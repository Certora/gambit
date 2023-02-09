import os
import sys
import subprocess
from pathlib import Path

MUTATIONS = [
    "BinaryOpMutation",
    "RequireMutation",
    "AssignmentMutation",
    "DeleteExpressionMutation",
    "FunctionCallMutation",
    "IfStatementMutation",
    "SwapArgumentsFunctionMutation",
    "SwapArgumentsOperatorMutation",
    "SwapLinesMutation",
    "UnaryOperatorMutation",
    "ElimDelegateMutation",
]

BENCHMARKS = "./benchmarks"
SOL = "sol"
CONFIG = "benchmarks/config-jsons/sanity-config.json"
JSON = "json"

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
    for name in MUTATIONS:
        actual = os.listdir('out/benchmarks/{name}/')
        if not actual:
            print("{name} failed sanity check. No mutants produced.")
            sys.exit(1)
        actual = actual[0]
        expected = f'benchmarks/{name}/expected.{SOL}'
        diff_invocation = ["diff", actual, expected]
        diff = subprocess.run(diff_invocation, capture_output=True, text=True)
        if diff.returncode == 0: # files are same
            print(f'{name} passed sanity check.')
        elif diff.returncode == 1: # files are different
            diff_file = open(f'out/{name}.{DIFF}', 'w')
            diff_file.write(subprocess.stdout)
            diff_file.close()
            print(f'{name} failed sanity check. See diff at out/{name}.{DIFF}')
        else:
            print("The `diff` subprocess failed to run. Install a `diff` program and try again")
            sys.exit(diff.returncode)
        
def main():
    update()
    mutate()
    # compare()
    # clean()

if __name__ == "__main__":
    main()
