"""`python -m idiomatic_sh_tastemaker`: runs the tastemaker executable with the same arguments."""

import os
import subprocess
import sys

from idiomatic_sh_tastemaker import find_tastemaker_bin


def main() -> None:
    """Replace this process with tastemaker, or run it and pass its exit status on Windows."""
    executable = find_tastemaker_bin()
    if sys.platform == "win32":
        sys.exit(subprocess.call([executable, *sys.argv[1:]]))
    os.execv(executable, [executable, *sys.argv[1:]])


if __name__ == "__main__":
    main()
