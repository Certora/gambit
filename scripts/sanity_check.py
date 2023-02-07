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
import os

BENCHMARKS = "./benchmarks"
SOL = "sol"
CONFIG = "config"
JSON = "json"

def update():
    for name in MUTATIONS:
        sol_file = f'benchmarks/{name}/{name}.{SOL}'
        outdir = f'benchmarks/{name}/'
        solc_invocation = [
            "solc",
            "--ast-compact-json",
            "--overwrite",
            sol_file,
            "-o",
            outdir
        ]
        os.system(" ".join(solc_invocation))
        solc_output = f'benchmarks/{name}/{name}.sol_json.ast'
        ast_json = f'benchmarks/{name}/{name}.json'
        mv_invocation = ["mv", solc_output, ast_json]
        os.system(" ".join(mv_invocation))

def mutate():
    config_file = "benchmarks/config-jsons/sanity-config.json"
    gambit_invocation = [
        "gambit",
        "mutate",
        "--json",
        config_file
    ]
    os.system(" ".join(gambit_invocation))

def compare():
    pass # TODO

def clean():
    os.system("rm -rf out")
        
def main():
    update()
    mutate()
    compare()
    # clean()

if __name__ == "__main__":
    main()
