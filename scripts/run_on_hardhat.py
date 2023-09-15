from argparse import ArgumentParser, Namespace
from collections import namedtuple
import shutil
import os
from os import path as osp
import json
import sys


def parse_args() -> Namespace:
    parser = ArgumentParser()
    parser.add_argument("project_dir")
    parser.add_argument("source_roots", nargs="*")
    parser.add_argument("--outdir", default="mutations")
    parser.add_argument("--mutations", default=None, nargs="*")
    parser.add_argument("--package_json", "-p", default="package.json")
    parser.add_argument("--gambit", default="gambit_out")
    parser.add_argument("--import_paths", "-I", default=None, nargs="*")
    parser.add_argument("--import_maps", "-m", default=None, nargs="*")
    args = parser.parse_args()
    return args

def collect_sources(source_roots):
    # Collect all solidity files from each source root in source_roots
    solidity_files = []
    for source_root in source_roots:
        for root, dirs, files in os.walk(source_root):
            for file in files:
                if file.endswith(".sol"):
                    print(f"Found solidity file {file} in {source_root}")
                    solidity_files.append(osp.join(root, file))
    return solidity_files

def parse_package_json(package_json):
    if not osp.exists(package_json):
        print(f"Error: package.json file {package_json} does not exist")
        sys.exit(1)
    with open(package_json, "r") as f:
        package = json.load(f)
    dependencies = package["dependencies"]
    dev_dependencies = package["devDependencies"]
    return namedtuple("Package", ["dependencies", "dev_dependencies"])(dependencies, dev_dependencies)
    
def resolve_dependencies(package, dependency_root="node_modules"):
    """
    Resolve all dependencies in package, returning a list of all dependency
    directories.
    """
    dependencies = package.dependencies
    dev_dependencies = package.dev_dependencies
    all_dependencies = {**dependencies, **dev_dependencies}
    dependency_dirs = []
    for dependency in all_dependencies:
        dependency_dir = osp.join(dependency_root, dependency)
        if not osp.exists(dependency_dir):
            print(f"Warning: dependency {dependency} does not exist in {dependency_dir}")
            continue
        dependency_dirs.append(namedtuple("Dependency", ["name", "dir", "remap"])(dependency, dependency_dir, f"{dependency}={dependency_dir}"))
    for d in dependency_dirs:
        print(f"Dependency {d.name} found at {d.dir}")
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
        import_paths = ["."]

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

def write_gambit_conf(gambit_conf, name="gambit.conf"):
    with open(name, "w") as f:
        json.dump(gambit_conf, f, indent=2)

def run_gambit(project_dir, source_roots, mutations, outdir, import_paths, import_maps):
    # Resolve outdir by making it absolute to the CWD
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
    gambit_conf = make_gambit_conf(sources, dependencies, mutations, outdir, import_paths, import_maps)
    write_gambit_conf(gambit_conf)


def main():
    args = parse_args()

    project_dir = args.project_dir
    source_roots = args.source_roots
    if len(source_roots) == 0:
        source_roots = ['contracts']
    
    run_gambit(project_dir, source_roots, args.mutations, args.outdir, args.import_paths, args.import_maps)

main()

