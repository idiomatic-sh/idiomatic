"""Independent import semantics; original dotted-directory fixture plus controls."""
from pathlib import Path
import importlib
import json
import sys
import tempfile

cases = [
    ("original-nested-dotted", "inner.name", "outer.inner.name"),
    ("valid-identifier-control", "inner_name", "outer.inner_name"),
    ("uppercase-style-control", "BadName", "outer.BadName"),
]
observations = []
for label, directory, module in cases:
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary) / "inventory"
        (root / "outer" / directory).mkdir(parents=True)
        (root / "outer/__init__.py").write_text("")
        (root / "outer" / directory / "__init__.py").write_text("")
        sys.path.insert(0, str(root))
        importlib.invalidate_caches()
        try:
            parent = importlib.import_module("outer")
            try:
                imported = importlib.import_module(module)
                result = {"imported": True, "file": str(Path(imported.__file__).relative_to(root))}
            except Exception as error:
                result = {"imported": False, "error_type": type(error).__name__, "message": str(error)}
            observations.append({"case": label, "logical_module": module,
                                 "parent_imported": parent is not None, "result": result})
        finally:
            sys.path.pop(0)
            for name in list(sys.modules):
                if name == "outer" or name.startswith("outer."):
                    del sys.modules[name]
print(json.dumps({"interpreter": sys.version, "observations": observations}, indent=2))
