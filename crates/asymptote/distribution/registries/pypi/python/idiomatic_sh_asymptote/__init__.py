"""The asymptote command line, installed as a native executable by this wheel."""

import os
import sysconfig


def find_asymptote_bin() -> str:
    """Return the path of the asymptote executable installed beside this package."""
    executable = "asymptote" + (sysconfig.get_config_var("EXE") or "")
    library = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))
    # The scheme whose library holds this package installed its scripts; a --target install keeps them beside it.
    beside = [
        sysconfig.get_path("scripts", scheme)
        for scheme in sysconfig.get_scheme_names()
        if library in {os.path.realpath(sysconfig.get_path(kind, scheme)) for kind in ("purelib", "platlib")}
    ]
    candidates = [
        *beside,
        os.path.join(library, "bin"),
        os.path.join(library, "Scripts"),
        sysconfig.get_path("scripts", sysconfig.get_default_scheme()),
        sysconfig.get_path("scripts", sysconfig.get_preferred_scheme("user")),
    ]
    for scripts in candidates:
        path = os.path.join(scripts, executable)
        if os.path.isfile(path):
            return path
    raise FileNotFoundError(executable)
