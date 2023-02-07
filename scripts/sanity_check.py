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
CONFIG = "conf"
JSON = "json"

def main():
    for name in MUTATIONS:
        sol_path = Path(BENCHMARKS) / name / f'{name}.{SOL}'
        sol_file = open(sol_path, 'r')
        print(sol_path)
        

if __name__ == "__main__":
    main()
