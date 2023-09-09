#!/usr/bin/env python3

"""
Generate RTD version of the Gambit README
"""

from argparse import ArgumentParser
from typing import Optional
import re


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


def is_suppress(line: str) -> bool:
    return "<!--SUPPRESS-->" == line.upper().replace(" ", "").strip()


def is_end_suppress(line: str) -> bool:
    return "<!--ENDSUPPRESS-->" == line.upper().replace(" ", "").strip()


def is_emit(line: str) -> bool:
    return "<!--EMIT:" == line.upper().replace(" ", "").strip()


def is_escaped_open_comment(line: str) -> bool:
    return line.strip() == r"<\!--"


def is_note_end(line: str) -> bool:
    """
    A note ends when a line is ended by an underscore. We double check to ensure
    that the line doesn't end with two underscores.
    """
    l = line.strip()
    if l.endswith("_"):
        return len(l) == 1 or l[-2] != "_"


def is_escaped_closed_comment(line: str) -> bool:
    return line.strip() == r"--\>"


def translate_readme_to_rtd(readme_file_path: str) -> str:
    with open(readme_file_path) as f:
        original = f.read()
        lines = original.split("\n")
    lines2 = []

    suppress_start = -1  # Track if we are suppressing
    note_start = -1  # Track if we've started a note
    emit_start = -1
    for i, line in enumerate(lines):
        # First, check if we are suppressing
        if suppress_start > -1:
            if is_end_suppress(line):
                suppress_start = -1
            continue

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

        elif note_start > -1 and is_note_end(line):
            note_start = -1
            lines2.append(line.rstrip().rstrip("_"))
            lines2.append("```")

        elif is_emit(line):
            if emit_start > 0:
                raise RuntimeError(
                    f"Cannot start a new emit on line {i+1}: already in an emit started at line {emit_start+1}"
                )
            emit_start = i

        elif line.strip() == "-->" and emit_start > -1:
            emit_start = -1

        # Handle escaped comments from inside of an emit
        elif is_escaped_open_comment(line) and emit_start > -1:
            lines2.append("<!--")
        elif is_escaped_closed_comment(line) and emit_start > -1:
            lines2.append("-->")

        elif is_suppress(line):
            if suppress_start > 0:
                raise RuntimeError(
                    f"Cannot start a new suppression on line {i+1}: already in a suppression tag started at line {suppress_start+1}"
                )
            suppress_start = i
        elif is_end_suppress(line):
            raise RuntimeError(
                f"Illegal end suppress on line {i+1}: not currently in a suppress"
            )
        else:
            # replace internal links
            l = replace_internal_references(line)
            lines2.append(l.strip("\n"))
    return "\n".join(lines2) + "\n"


def main():
    parser = ArgumentParser()
    parser.add_argument("readme_file", help="README.md file to translate to RTD")
    parser.add_argument("--output", "-o", default="gambit.md", help="output file")

    args = parser.parse_args()
    rtd = translate_readme_to_rtd(args.readme_file)
    with open(args.output, "w+") as f:
        print("Writing to", args.output)
        f.write(rtd)


if __name__ == "__main__":
    main()
