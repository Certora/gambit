#!/usr/bin/env python3

"""
Generate RTD version of the Gambit README
"""

from argparse import ArgumentParser
from typing import Optional
import re
import hashlib


def line_is_anchor(line: str) -> bool:
    return line.startswith("<!-- ANCHOR:")


def replace_internal_references(line):
    """
    Internal references look like '[Readable Text](#some-header-title)'. We
    want to repalce this with '{ref}`some-header-title`

    Examples
    ========

    >>> replace_internal_references("[Test1](#test1")
    '[Test1](#test1'
    >>> replace_internal_references("[Test1](#test1)")
    '{ref}`test1`'
    >>> replace_internal_references("This is [Test2](#test-2) with [Multiple Matches](#multiple-matches)")
    'This is {ref}`test-2` with {ref}`multiple-matches`'
    """
    pat = r"\[([^\]]+)\]\(#([^\)]+)\)"
    while True:
        m = re.search(pat, line)
        if m is None:
            return line
        line = line[: m.start()] + "{ref}" + f"`{m.group(2)}`" + line[m.end() :]


def get_anchor(line: str) -> Optional[str]:
    """
    Attempt to get an anchor from a comment

    Examples
    ========

    >>> get_anchor("<!-- ANCHOR: (test-anchor)= -->")
    '(test-anchor)='
    >>> get_anchor("# README")
    """

    if not line.startswith("<!-- ANCHOR: "):
        return None
    line = line.strip()
    anchor = line[len("<!-- ANCHOR:") : -len("-->")].strip()
    return anchor


def translate(readme_file_path: str) -> str:
    with open(readme_file_path) as f:
        original = f.read()
        lines = original.split("\n")
    lines2 = []

    note_start = -1  # Track if we've started a note
    for i, line in enumerate(lines):
        anchor = get_anchor(line)
        if anchor is not None:
            lines2.append(anchor)
        elif "_**note:**" == line.strip().lower():
            if note_start > -1:
                raise RuntimeError(
                    f"Already in note from line {note_start + 1}, cannot start new note on line {i+1}"
                )
            note_start = i
            lines2.append("```{note}")
        elif "_**note:**" in line.strip().lower():
            raise RuntimeError(
                f"Illegal note start on line {i+1}: new note tags '_**Note:**' and their closing '_' must be on their own lines"
            )

        elif note_start > -1 and line.strip() == "_":
            note_start = -1
            lines2.append("```")
        else:
            # replace internal links
            l = replace_internal_references(line)
            lines2.append(l.strip("\n"))
    signature = hashlib.md5(original.encode()).hexdigest()
    lines2.append(f"<!-- signature: {signature} -->")
    return "\n".join(lines2) + "\n"


def main():
    parser = ArgumentParser()
    parser.add_argument("readme_file", help="README.md file to translate to RTD")
    parser.add_argument("--output", "-o", default="gambit.md", help="output file")

    args = parser.parse_args()
    rtd = translate(args.readme_file)
    with open(args.output, "w+") as f:
        print("Writing to", args.output)
        f.write(rtd)


if __name__ == "__main__":
    main()
