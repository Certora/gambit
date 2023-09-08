#!/usr/bin/env python3

from argparse import ArgumentParser
from urllib.request import urlopen
import difflib
import os
import sys
import re
from generate_rtd_markdown import translate_readme_to_rtd

OUR_README_PATH = os.path.join(os.path.dirname(__file__), "..", "README.md")

THEIR_README_URL_NO_BRANCH = (
    "https://raw.githubusercontent.com/Certora/Documentation/{}/docs/gambit/gambit.md"
)


def main():
    parser = ArgumentParser()
    parser.add_argument(
        "--branch", default="master", help="Branch to check README from"
    )
    args = parser.parse_args()

    exit_code = check_rtd_docs_up_to_date(branch=args.branch)

    sys.exit(exit_code)


def check_rtd_docs_up_to_date(branch="master") -> int:
    url = THEIR_README_URL_NO_BRANCH.format(branch)

    with open(OUR_README_PATH) as f:
        our_readme_contents = f.read()

    try:
        their_readme_contents = urlopen(url).read().decode("utf-8")

    except RuntimeError as e:
        print(f"Could not read `gambit.md` from {url}")
        print(f"Error: {e}")
        return 127

    print()
    if our_readme_contents == their_readme_contents:
        print(f"Docs are in sync!")
        return 0
    else:
        print(f"Docs are out of sync!")
        print()
        our_translated_readme_contents = translate_readme_to_rtd(OUR_README_PATH)
        print("Unified diff: Local vs Remote")
        print("=============================")
        print()
        print(
            "".join(
                difflib.unified_diff(
                    our_translated_readme_contents.splitlines(keepends=True),
                    their_readme_contents.splitlines(keepends=True),
                )
            )
        )
        return 1


if __name__ == "__main__":
    main()
