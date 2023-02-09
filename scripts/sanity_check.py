import os
import sys
import subprocess
from pathlib import Path

MUTATIONS = [
    "AssignmentMutation",
    "BinaryOpMutation",
    "DeleteExpressionMutation",
    "ElimDelegateMutation",
    "FunctionCallMutation",
    "IfStatementMutation",
    "RequireMutation",
    "SwapArgumentsFunctionMutation",
    "SwapArgumentsOperatorMutation",
    "SwapLinesMutation",
    "UnaryOperatorMutation",
]

BENCHMARKS = "./benchmarks"
SOL = "sol"
CONFIG = "benchmarks/config-jsons/sanity-config.json"
JSON = "json"
DIFF = "diff"

def update():
    for name in MUTATIONS:
        sol_file = f'benchmarks/{name}/{name}.{SOL}'
        ast_json = f'benchmarks/{name}/{name}.{JSON}'
        ast_file = open(ast_json, 'w')
        solc_invocation = [
            "solc",
            "--ast-compact-json",
            "--overwrite",
            sol_file,
        ]
        subprocess.run(solc_invocation, stdout=ast_file)
        ast_file.close()

def mutate():
    gambit_invocation = [
        "gambit",
        "mutate",
        "--json",
        CONFIG,
    ]
    subprocess.run(gambit_invocation)

def compare():
    succeeded = 0
    for name in MUTATIONS:
        print(f'Running sanity check for {name}...')
        actual = os.listdir(f'out/benchmarks/{name}/')
        if not actual:
            print("FAIL: no mutants produced")
            continue
        actual = f'out/benchmarks/{name}/{actual[0]}'
        expected = f'benchmarks/{name}/expected.{SOL}'
        diff_invocation = ["diff", actual, expected]
        diff = subprocess.run(diff_invocation, capture_output=True, text=True)
        if diff.returncode == 0: # files are same
            print("SUCCESS")
            succeeded += 1
        elif diff.returncode == 1: # files are different
            diff_file = open(f'out/{name}.{DIFF}', 'w')
            diff_file.write(diff.stdout)
            diff_file.close()
            print(f'FAIL: output did not match expected. See diff at out/{name}.{DIFF}')
        else:
            print(f'The `diff` subprocess failed to run on {name}. Check for missing files or install a `diff` program and try again')
            sys.exit(diff.returncode)
    print(f'Sanity check finished with {succeeded} of {len(MUTATIONS)} succeeded.')
        
def main():
    update()
    mutate()
    compare()

if __name__ == "__main__":
    main()
