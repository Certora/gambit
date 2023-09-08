#!/usr/bin/env python3

from argparse import ArgumentParser
from urllib.request import urlopen
import difflib
import hashlib
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


def find_signature(contents: str) -> str:
    pattern = r"<!--\s*signature:\s*([0-9a-fA-F]{32})\s*-->"
    m = re.search(pattern, contents)
    if m is None:
        return None
    return m.group(1)


def check_rtd_docs_up_to_date(branch="master") -> int:
    url = THEIR_README_URL_NO_BRANCH.format(branch)

    with open(OUR_README_PATH) as f:
        our_readme_contents = f.read()

    our_md5 = hashlib.md5(our_readme_contents.encode()).hexdigest()

    try:
        their_readme_contents = urlopen(url).read()
        their_md5 = find_signature(their_readme_contents.decode("utf-8"))

    except RuntimeError as e:
        print(f"Could not read `gambit.md` from {url}")
        print(f"Error: {e}")
        return 127

    print("local md5: ", our_md5)
    print("remote md5:", their_md5)
    print()
    if our_md5 == their_md5:
        print(f"MD5 Hashes Match: Documentation is synced")
        return 0
    else:
        print(f"MD5 Hashes Do Not Match!")
        print()
        our_translated_readme_contents = translate_readme_to_rtd(OUR_README_PATH)
        print("Unified diff: Local vs Remote")
        print("=============================")
        print()
        print(
            "".join(
                difflib.unified_diff(
                    our_translated_readme_contents.splitlines(keepends=True),
                    str(their_readme_contents.decode("utf-8")).splitlines(
                        keepends=True
                    ),
                )
            )
        )
        return 1


if __name__ == "__main__":
    main()
