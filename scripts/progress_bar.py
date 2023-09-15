import sys
from typing import Optional


def progress_bar(
    it, prefix: str = "", size: int = 60, inc: Optional[int] = None, out=sys.stdout
):
    """
    Print a progress bar

    Taken from https://stackoverflow.com/a/34482761
    """
    count = len(it)
    if inc is None:
        inc = 1
        if count > 1000:
            inc = count // 100

    if count == 0:
        print(
            f"{prefix}[{u'█'*size}] {0}/{0} ({100.0:3.2f}%)",
            file=out,
            flush=True,
        )
        return

    def show(j):
        x = int(size * j / count)
        print(
            f"{prefix}[{u'█'*x}{('.'*(size-x))}] {j}/{count} ({100*j/count:3.2f}%)",
            end="\r",
            file=out,
            flush=True,
        )

    show(0)
    for i, item in enumerate(it):
        yield item
        if (i + 1) % inc == 0:
            show(i + 1)
    print("\n", flush=True, file=out)
