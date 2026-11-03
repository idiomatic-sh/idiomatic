"""The `placement` step done through GitHub, with `gh`."""

from __future__ import annotations

import base64
import json
import subprocess
from pathlib import Path

from ci.release_pr.placement.interface import Placement


class GithubPlacement(Placement):
    """`placement` through GitHub's contents API with `gh`, for the repository the checkout's remote names: the file read from the
    branch, and committed onto it over the blob it replaces."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def contents(self, product: str, branch: str) -> dict:
        """The contents API's answer for the product's `tastemaker.toml` on `branch`: its text, base64, and its blob."""
        shown = subprocess.run(
            ["gh", "api", f"repos/{{owner}}/{{repo}}/contents/crates/{product}/tastemaker.toml?ref={branch}"],
            cwd=self.root,
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
        ).stdout
        return json.loads(shown)

    def read(self, product: str, branch: str) -> str:
        return base64.b64decode(self.contents(product, branch)["content"]).decode("utf-8")

    def write(self, product: str, branch: str, text: str, message: str) -> None:
        blob = self.contents(product, branch)["sha"]
        subprocess.run(
            [
                "gh", "api", "--method", "PUT", f"repos/{{owner}}/{{repo}}/contents/crates/{product}/tastemaker.toml",
                "-f", f"message={message}",
                "-f", f"content={base64.b64encode(text.encode('utf-8')).decode('ascii')}",
                "-f", f"sha={blob}",
                "-f", f"branch={branch}",
            ],
            cwd=self.root,
            check=True,
            capture_output=True,
        )  # fmt: skip
