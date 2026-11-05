import hashlib
import io
import json
import os
from pathlib import Path
import tempfile

root = Path(__file__).parent
inputs = json.loads((root / "independent-io-inputs.json").read_text())
results = []
for case in inputs:
    source = case["source"]
    assert hashlib.sha256(source.encode()).hexdigest() == case["source_sha256"]
    namespace = {"rows": ["z"]}
    if case["name"] == "io-binding-2":
        with tempfile.TemporaryDirectory() as directory:
            old = os.getcwd()
            try:
                os.chdir(directory)
                Path("x").write_text("line\n")
                try:
                    exec(compile(source, "case.py", "exec", dont_inherit=True), namespace)
                    error = None
                except Exception as failure:
                    error = {"type": type(failure).__name__, "message": str(failure)}
                descriptor = namespace.get("f")
                results.append(
                    {
                        "name": case["name"],
                        "source_sha256": case["source_sha256"],
                        "receiver_type": type(descriptor).__name__,
                        "has_write": hasattr(descriptor, "write"),
                        "error": error,
                    }
                )
                if isinstance(descriptor, int):
                    os.close(descriptor)
            finally:
                os.chdir(old)
        continue
    exec(compile(source, "case.py", "exec", dont_inherit=True), namespace)
    stream = namespace["stream"] if "stream" in namespace else namespace["f"]
    results.append(
        {
            "name": case["name"],
            "source_sha256": case["source_sha256"],
            "receiver_type": type(stream).__name__,
            "is_io_base": isinstance(stream, io.IOBase),
            "is_text_io_base": isinstance(stream, io.TextIOBase),
            "has_writelines": hasattr(stream, "writelines"),
            "written": stream.getvalue(),
        }
    )
print(
    json.dumps(
        {
            "python": __import__("sys").version,
            "cases": results,
            "scope": "Exact source with only rows and temporary file environment framed. No claim about arbitrary custom IO implementations or universal fix equivalence.",
        },
        indent=2,
    )
)
