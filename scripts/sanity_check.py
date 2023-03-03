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

BENCHMARKS = "benchmarks"
SOL = "sol"
CONFIG = "benchmarks/config-jsons/sanity-config.json"
JSON = "json"
DIFF = "diff"
OUTDIR = "out"
EXPECTED = "expected"
MUTANTS = "mutants"

def mutate() -> None:
    gambit_invocation = [
        "gambit",
        "mutate",
        "--json",
        CONFIG,
    ]
    comp_proc = subprocess.run(gambit_invocation)
    if comp_proc.returncode != 0:
        print("FAIL: Gambit failed unexpectedly")
        sys.exit(comp_proc.returncode)

def compare() -> None:
    succeeded = 0
    for name in MUTATIONS:
        mutant_parent = Path(OUTDIR) / MUTANTS
        mutant_abs = Path(BENCHMARKS).absolute() / name
        for part in mutant_abs.parts[1:]:
            mutant_parent = Path.joinpath(mutant_parent, part)
        print(f'Running sanity check for {name}...')
        actual = os.listdir(mutant_parent)
        if not actual:
            print("FAIL: no mutants produced")
            continue
        actual = mutant_parent / (actual[0])
        expected = Path(BENCHMARKS) / name / f'{EXPECTED}.{SOL}'
        diff_invocation = [DIFF, actual, expected]
        diff = subprocess.run(diff_invocation, capture_output=True, text=True)
        if diff.returncode == 0: # files are same
            print("SUCCESS")
            succeeded += 1
        elif diff.returncode == 1: # files are different
            diff_path = Path(OUTDIR) / f'{name}.{DIFF}'
            with open(diff_path, 'w') as diff_file:
                diff_file.write(diff.stdout)
                print(f'FAIL: output did not match expected. See diff at {diff_path}')
        else:
            print(f'The `diff` subprocess failed to run on {name}. Check for missing files or install a `diff` program and try again')
            sys.exit(diff.returncode)
    print(f'Sanity check finished with {succeeded} of {len(MUTATIONS)} succeeded.')
    if (succeeded < len(MUTATIONS)):
        sys.exit(1)
        
def main() -> None:
    mutate()
    compare()

if __name__ == "__main__":
    main()
