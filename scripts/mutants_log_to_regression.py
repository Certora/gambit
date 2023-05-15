"""
Read in a mutants.log file from gambit_out and output a regression test to be added to integration_tests.rs
"""
from os import path as osp


def read_test(json):
    with open(osp.join("gambit_out", "mutants.log")) as f:
        lines = f.readlines()
    testcases = []
    for line in lines:
        mid, op, filepath, linecol, orig, repl = line.strip().split(",")
        line, col = linecol.split(":")
        orig = orig.strip()
        repl = repl.strip()
        testcase = f'("{op}", "{orig}", "{repl}", ({line}, {col})),'
        testcases.append(testcase)

    testcases = "\n            ".join(testcases)

    if json:
        print(
            f"""    assert_exact_mutants_from_json(
        "{json}",
        &vec![
            {testcases}
        ],
    )"""
        )
    else:
        print(f"            {testcases}")


def main():
    from argparse import ArgumentParser

    parser = ArgumentParser()
    parser.add_argument(
        "json",
        nargs="?",
        default=None,
        help="Json file name to run test on (filename only! this should exist inside of benchmarks/config-jsons)",
    )

    args = parser.parse_args()

    read_test(args.json)


if __name__ == "__main__":
    main()
