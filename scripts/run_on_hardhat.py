#!/usr/bin/env python3

from argparse import ArgumentParser, Namespace
from collections import namedtuple
import shutil
import os
from os import path as osp
import json
import sys
from typing import List
import subprocess
import progress_bar

class ToolRunData:
    def __init__(self, tool_name, import_paths, import_maps, sources):
        self.tool_name = tool_name
        self.import_paths = import_paths
        self.import_maps = import_maps
        self.sources = sources
        self.successes = []
        self.failures = []
        self.outputs = {}

    def add_success(self, source, stderr, stdout):
        self.successes.append(source)
        self.outputs[source] = (stderr.decode('utf-8'), stdout.decode('utf-8'))
    
    def add_failure(self, source, stderr, stdout):
        self.failures.append(source)
        self.outputs[source] = (stderr.decode('utf-8'), stdout.decode('utf-8'))
    
    def write_to_disk(self, outdir="data_collect"):
        if not osp.exists(outdir):
            os.makedirs(outdir)
        tool_out_dir = osp.join(outdir, self.tool_name)
        if osp.exists(tool_out_dir):
            shutil.rmtree(tool_out_dir)
        os.makedirs(tool_out_dir)
        outputs_dir = osp.join(tool_out_dir, "outputs")
        os.makedirs(outputs_dir)

        with open(osp.join(tool_out_dir, "successes.txt"), "w") as f:
            f.write("\n".join(self.successes))

        with open(osp.join(tool_out_dir, "failures.txt"), "w") as f:
            f.write("\n".join(self.failures))
        
        for source in self.outputs:
            # Replace slashes with dots in source name
            source_no_slashes = source.replace("/", ".")
            stderr, stdout = self.outputs[source]
            stderr_path = osp.join(outputs_dir, source_no_slashes + ".stderr")
            stdout_path = osp.join(outputs_dir, source_no_slashes + ".stdout")
            with open(stderr_path, "w") as f:
                f.write(stderr)
            with open(stdout_path, "w") as f:
                f.write(stdout)
        
        with open(osp.join(tool_out_dir, "conf.json"), "w") as f:
            conf = {
                "import_paths": self.import_paths,
                "import_maps": self.import_maps,
                "sources": self.sources,
            }
            json.dump(conf, f, indent=2)
        
        with open(osp.join(outdir, f"{self.tool_name}_run_data.json"), "w") as f:
            data = {
                "tool_name": self.tool_name,
                "import_paths": self.import_paths,
                "import_maps": self.import_maps,
                "sources": self.sources,
                "successes": self.successes,
                "failures": self.failures,
                "num_sources": len(self.sources),
                "num_successes": len(self.successes),
                "num_failures": len(self.failures),
                "success_rate": len(self.successes) / len(self.sources),
            }
            json.dump(data, f, indent=2)

    def print_summary(self):
        successes = self.successes
        failures = self.failures
        total = len(successes) + len(failures)
        if successes:
            print(f"Successes for {self.tool_name}:")
            for source in successes:
                print(f"    [\033[32;1m + \033[0m] {source}")
        if failures:
            print(f"Failures:")
            for source in failures:
                print(f"    [\033[31;1m - \033[0m] {source}")
        print(f"    {len(successes)} / {total} successes ({len(successes) / total * 100:.2f}%)")
        print(f"    {len(failures)} / {total} failures ({len(failures) / total * 100:.2f}%)")


def warning(msg):
    print(f"\033[33;1mWarning\033[0m: {msg}")

def error(msg):
    print(f"\033[31;1mError\033[0m: {msg}")

def info(msg):
    print(f"\033[34;1mInfo\033[0m: {msg}")

def parse_args() -> Namespace:
    parser = ArgumentParser()
    parser.add_argument("project_dir")
    parser.add_argument("source_roots", nargs="*", help="Roots of directories where solidity code is located")
    parser.add_argument("--outdir", default="gambit_out")
    parser.add_argument("--mutations", default=None, nargs="*")
    parser.add_argument("--package_json", "-p", default="package.json")
    parser.add_argument("--import_paths", "-I", default=None, nargs="*")
    parser.add_argument("--import_maps", "-m", default=None, nargs="*")
    parser.add_argument("--gambit", action="store_true", help="Run gambit on project")
    parser.add_argument("--solc", default=None, help="solc executable to use with the given configuration (by default, solc is not run)")
    parser.add_argument("--solang_parser", action="store_true", help="Run solang_parser on project (get from https://github.com/bkushigian/SolangParse4Debugging)")
    parser.add_argument("--halt_on_failure", action="store_true", help="Halt on first failure")
    parser.add_argument("--collect_data", action="store_true", help="Collect data on tool runs")
    args = parser.parse_args()
    return args

def default_import_paths():
    return [".", "contracts"]

def collect_sources(source_roots):
    info(f"Collecting Solidity sources from roots: {' ,'.join(source_roots)}")
    # Collect all solidity files from each source root in source_roots
    solidity_files = []
    for source_root in source_roots:
        for root, dirs, files in os.walk(source_root):
            for file in files:
                if file.endswith(".sol"):
                    info(f"Found solidity file {file} in {source_root}")
                    solidity_files.append(osp.join(root, file))
    info(f"Found {len(solidity_files)} solidity files")
    return solidity_files

def parse_package_json(package_json):
    if not osp.exists(package_json):
        error(f"package.json file {package_json} does not exist")
        sys.exit(1)
    with open(package_json, "r") as f:
        package = json.load(f)
    dependencies = package["dependencies"] if "dependencies" in package else {}
    dev_dependencies = package["devDependencies"] if "devDependencies" in package else {}
    return namedtuple("Package", ["dependencies", "dev_dependencies"])(dependencies, dev_dependencies)
    
def resolve_dependencies(package, dependency_root="node_modules"):
    """
    Resolve all dependencies in package, returning a list of all dependency
    directories.
    """
    dependencies = package.dependencies
    dev_dependencies = package.dev_dependencies
    info(f"Found {len(dependencies)} dependencies and {len(dev_dependencies)} dev dependencies")
    all_dependencies = {**dependencies, **dev_dependencies}
    dependency_dirs = []
    print()
    info(f"Resolving {len(all_dependencies)} dependencies in {dependency_root}")
    for dependency in all_dependencies:
        dependency_dir = osp.join(dependency_root, dependency)
        if not osp.exists(dependency_dir):
            warning(f"dependency {dependency} does not exist in {dependency_dir}")
            continue
        dependency_dirs.append(namedtuple("Dependency", ["name", "dir", "remap"])(dependency, dependency_dir, f"{dependency}={dependency_dir}"))
    for d in dependency_dirs:
        info(f"Dependency {d.name} found at {d.dir}")
    print()
    return dependency_dirs

def make_gambit_conf(sources, dependencies, mutations, outdir, import_paths, import_maps):
    """
    Create a gambit configuration file for the given sources, dependencies,
    mutations, and output directory.
    """
    # Create the gambit configuration file. It is a JSON file containing an
    # array of objects, where each object corresponds to a single source file's
    # mutations
    gambit_conf = []

    if import_maps is None:
        import_maps = []
    for d in dependencies: 
        import_maps.append(d.remap)

    if import_paths is None or import_paths == []:
        import_paths = default_import_paths()

    for source in sources:
        source_conf = {
            "filename": source,
            "outdir": outdir,
            "import_maps": import_maps,
            "import_paths": import_paths,
        }
        if mutations is not None:
            source_conf["mutations"] = mutations
        gambit_conf.append(source_conf)
    return gambit_conf

def make_gambit_args(source, import_paths, import_maps, mutations=None, outdir=None):

    if import_paths is None or import_paths == []:
        import_paths = default_import_paths()

    if import_maps is None:
        import_maps = []


    gambit_args = ["mutate", source]
    if import_paths is not None and import_paths != []:
        gambit_args.append("--import_paths")
        for import_path in import_paths:
            gambit_args.append(import_path)
        
    if import_maps is not None and import_maps != []:
        gambit_args.append("--import_maps")
        for import_map in import_maps:
            gambit_args.append(import_map)

    if mutations is not None:
        gambit_args.append("--mutations")
        for mutation in mutations:
            gambit_args.append(mutation)
    
    if outdir is not None:
        gambit_args.append("--outdir")
        gambit_args.append(outdir)
    return gambit_args

def make_solc_args(sources=None, import_paths=None, import_maps=None, allow_paths=None):
    """
    Create a list of solc arguments for the given compilation configuration
    """
    if sources is None:
        sources = []
    if isinstance(sources, str):
        sources = [sources]

    if import_paths is None or import_paths == []:
        import_paths = default_import_paths()
    base_path, include_paths = import_paths[0], import_paths[1:]

    if import_maps is None:
        import_maps = []

    if allow_paths is None or allow_paths == []:
        allow_paths = None

    solc_args = []
    for source in sources:
        solc_args.append(source)
    if base_path is not None:
        solc_args.append("--base-path")
        solc_args.append(base_path)
        if include_paths is not None:
            for include_path in include_paths:
                solc_args.append("--include-path")
                solc_args.append(include_path)
    if allow_paths is not None:
        solc_args.append("--allow-paths")
        solc_args.append(','.join(allow_paths))
    for import_map in import_maps:
        solc_args.append(import_map)
    return solc_args

def make_solang_parser_args(source, import_paths=None, import_maps=None, target="solana"):
    """
    Create a list of solang args for the given compilation configuration
    """
    if import_paths is None or import_paths == []:
        import_paths = default_import_paths()

    solang_args = [source]
    if import_paths is not None and import_paths != []:
        solang_args.append("--import_paths")
        for import_path in import_paths:
            solang_args.append(import_path)
    if import_maps is not None and import_maps != []:
        solang_args.append("--import_maps")
        for import_map in import_maps:
            solang_args.append(import_map)
    return solang_args


def write_gambit_conf(gambit_conf, name="gambit.conf"):
    path = osp.abspath(name)
    with open(path, "w") as f:
        json.dump(gambit_conf, f, indent=2)
    print("Wrote gambit conf to " + path)
    return path

def run_solc(project_dir, source_roots, import_paths, import_maps, solc="solc", halt_on_failure=False):
    print("\n === Running solc ===\n")
    curdir = os.getcwd()
    # Change directory to project_dir
    os.chdir(project_dir)
    sources = collect_sources(source_roots)
    package = parse_package_json("package.json")
    dependencies = resolve_dependencies(package)
    if import_maps is None:
        import_maps = []
    import_maps = [d.remap for d in dependencies] + import_maps
    if import_paths is None or import_paths == []:
        import_paths = default_import_paths()
    tool_run_data = ToolRunData(solc, import_paths, import_maps, sources)

    print(f"Running solc on {len(sources)} source files:\n")
    for source in progress_bar.progress_bar(sources):
        solc_args: List[str] = make_solc_args(source, import_paths, import_maps)
        # Run solc with provided args
        command = [solc, *solc_args]
        output = subprocess.run(command, capture_output=True)
        if output.returncode != 0:
            error(f"The following command failed to run on {source}")
            print(f"    {' '.join(command)}")
            print()
            print(f"\033[31;1mstderr:\033[0m {output.stderr.decode('utf-8')}")
            print()
            tool_run_data.add_failure(source, output.stderr, output.stdout)
            if halt_on_failure:
                print("Halting on fail")
                break
        else:
            tool_run_data.add_success(source, output.stderr, output.stdout)

    print(f"Finished running gambit on {len(sources)} source files")
    tool_run_data.print_summary()
    os.chdir(curdir)
    return tool_run_data


def run_solang_parser(project_dir, source_roots, import_paths, import_maps, solang_parser="solang_parser", halt_on_failure=False):
    print("\n === Running solang_parser ===\n")

    # Change directory to project_dir
    curdir = os.getcwd()
    os.chdir(project_dir)

    sources = collect_sources(source_roots)
    package = parse_package_json("package.json")
    dependencies = resolve_dependencies(package)

    if import_maps is None:
        import_maps = []
    import_maps = [d.remap for d in dependencies] + import_maps

    if import_paths is None or import_paths == []:
        import_paths = default_import_paths()

    # Now run solang_parser
    tool_run_data = ToolRunData("solang_parser", import_paths, import_maps, sources)

    print(f"Running solang_parser on {len(sources)} source files:\n")
    for source in progress_bar.progress_bar(sources):
        solang_args: List[str] = make_solang_parser_args(source=source, import_paths=import_paths, import_maps=import_maps)
        # Run solang_parser with provided args
        command = [solang_parser, *solang_args]
        output = subprocess.run(command, capture_output=True)
        if output.returncode != 0:
            print('--------------------------------------------------------------------------------')
            error(f"The following command failed to run on \033[36;1m{source}\033[0m\n")
            print(f"    {' '.join(command)}")
            print()
            print(f"stderr:\n{output.stderr.decode('utf-8')}")
            print()
            tool_run_data.add_failure(source, output.stderr, output.stdout)
            if halt_on_failure:
                print("Halting on fail")
                break
        else:
            tool_run_data.add_success
    print(f"Finished running solang_parser on {len(sources)} source files")
    tool_run_data.print_summary()
    os.chdir(curdir)
    return tool_run_data
    


def run_gambit(project_dir, source_roots, mutations, outdir, import_paths, import_maps, run_with_conf=False, halt_on_failure=False):
    print("\n === Running gambit ===\n")
    # Resolve outdir by making it absolute to the CWD
    curdir = os.getcwd()
    outdir = osp.abspath(outdir)
    if osp.exists(outdir):
        print(f"Output directory {outdir} already exists. Removing...")
        shutil.rmtree(outdir)
    # Now make the directory and all parent directories as needed
    os.makedirs(outdir)

    # Change directory to project_dir
    os.chdir(project_dir)

    sources = collect_sources(source_roots)
    package = parse_package_json("package.json")
    dependencies = resolve_dependencies(package)

    tool_run_data = ToolRunData("gambit", import_paths, import_maps, sources)
    if run_with_conf:
        gambit_conf = make_gambit_conf(sources, dependencies, mutations, outdir, import_paths, import_maps)
        path = write_gambit_conf(gambit_conf, name="gambit.conf")
        subprocess.run(["gambit", "mutate", "--json", path])
    else:
        print(f"Running gambit on {len(sources)} source files...")
        for source in progress_bar.progress_bar(sources):
            gambit_args = make_gambit_args(source, import_paths, import_maps, mutations=mutations, outdir=outdir)
            # Run gambit with provided args
            command = ["gambit", *gambit_args]
            output = subprocess.run(command, capture_output=True)
            if output.returncode != 0:
                error(f"Gambit failed on {source}")
                print(f"    {' '.join(command)}")
                stderr = output.stderr.decode('utf-8').strip()
                stdout = output.stdout.decode('utf-8').strip()
                if stdout or stderr:
                    print('---------------------------------')
                if stdout:
                    print(f"\033[31;1mstdout:\033[0m {output.stdout.decode('utf-8')}")
                if stderr:
                    print(f"\033[31;1mstderr:\033[0m {output.stderr.decode('utf-8')}")
                if stdout or stderr:
                    print('---------------------------------')
                tool_run_data.add_failure(source, output.stderr, output.stdout)
                if halt_on_failure:
                    print("Halting on failure")
                    break
            else:
                info("Successfully ran gambit on " + source)
                tool_run_data.add_success(source, output.stderr, output.stdout)
    tool_run_data.print_summary()
    os.chdir(curdir)
    return tool_run_data


def main():
    args = parse_args()

    project_dir = args.project_dir
    source_roots = args.source_roots
    if len(source_roots) == 0:
        source_roots = ['contracts']

    if args.solc:
        solc_run_data = run_solc(project_dir, source_roots, args.import_paths, args.import_maps, halt_on_failure=args.halt_on_failure, solc=args.solc)
        if args.collect_data:
            solc_run_data.write_to_disk()
    
    if args.solang_parser:
        print(args.import_maps)
        solang_parser_run_data = run_solang_parser(project_dir, source_roots, args.import_paths, args.import_maps, halt_on_failure=args.halt_on_failure)
        if args.collect_data:
            solang_parser_run_data.write_to_disk()

    if args.gambit:
        gambit_run_data = run_gambit(project_dir, source_roots, args.mutations, args.outdir, args.import_paths, args.import_maps, halt_on_failure=args.halt_on_failure)
        if args.collect_data:
            gambit_run_data.write_to_disk()


main()
