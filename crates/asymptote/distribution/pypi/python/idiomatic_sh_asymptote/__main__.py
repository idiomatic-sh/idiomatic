"""`python -m idiomatic_sh_asymptote`: runs the asymptote executable with the same arguments."""

import os
import subprocess
import sys

from idiomatic_sh_asymptote import find_asymptote_bin


def main() -> None:
    """Replace this process with asymptote, or run it and pass its exit status on Windows."""
    executable = find_asymptote_bin()
    if sys.platform == "win32":
        sys.exit(subprocess.call([executable, *sys.argv[1:]]))
    os.execv(executable, [executable, *sys.argv[1:]])


if __name__ == "__main__":
    main()
