import hashlib
import json
from pathlib import Path
import typing

root = Path(__file__).parent
data = json.loads((root / "late-annotation-input.json").read_text())
source = data["source"]
assert hashlib.sha256(source.encode()).hexdigest() == data["source_sha256"]
namespace = {}
exec(compile(source, data["file"], "exec", dont_inherit=True), namespace)
declared = typing.get_type_hints(namespace["f"])["fp"]
print(
    json.dumps(
        {
            "python": __import__("sys").version,
            "source_sha256": data["source_sha256"],
            "raw_annotation": namespace["f"].__annotations__["fp"],
            "resolved_module": declared.__module__,
            "resolved_name": declared.__qualname__,
            "has_readlines": hasattr(declared, "readlines"),
            "has_iter": hasattr(declared, "__iter__"),
            "scope": "Exact authored source compiled without inherited future flags; function body not invoked. The result establishes declared type only, not runtime argument enforcement.",
        },
        indent=2,
    )
)
