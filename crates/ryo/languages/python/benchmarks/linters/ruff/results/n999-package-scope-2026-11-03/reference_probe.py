"""Pinned Ruff explicit-file selection and package-name controls, no candidate."""
from pathlib import Path
import hashlib
import json
import subprocess
import tempfile
import sys

ruff = Path(sys.argv[1]).resolve()
controls = [
    ("explicit-txt-parser", "test.txt", "x= (\n", {}),
    ("explicit-extensionless-parser", "file-with-dashes", "x= (\n", {}),
    ("original-nested-dotted", "outer/inner.name/__init__.py", "", {"outer/__init__.py": ""}),
    ("valid-identifier-control", "outer/inner_name/__init__.py", "", {"outer/__init__.py": ""}),
    ("uppercase-style-control", "outer/BadName/__init__.py", "", {"outer/__init__.py": ""}),
]
observations = []
for name, filename, source, auxiliary in controls:
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary) / "inventory"
        root.mkdir()
        for relative, contents in {**auxiliary, filename: source}.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(contents)
        command = [str(ruff), "check", "--isolated", "--select", "N999", "--target-version", "py310",
                   "--output-format", "json", "--", filename]
        result = subprocess.run(command, cwd=root, capture_output=True, text=True)
        assert result.returncode in (0, 1), result.stderr
        findings = json.loads(result.stdout)
        for finding in findings:
            finding["filename"] = str(Path(finding["filename"]).relative_to(root))
        observations.append({"case": name, "files": {**auxiliary, filename: source},
                             "command": command, "returncode": result.returncode, "findings": findings})
print(json.dumps({"ruff_sha256": hashlib.sha256(ruff.read_bytes()).hexdigest(), "observations": observations}, indent=2))
