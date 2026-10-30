"""The `publish` step done through uv, PyPI's publish."""

from __future__ import annotations

import subprocess

from ci.release.publish.interface import Publish
from ci.release.use_case import Artifact, Product


def uv_publish(artifact: Artifact, dry_run: bool) -> list[str]:
    """uv's publish of one wheel: its dry run, or an upload through PyPI's Trusted
    Publishing, which only a workflow the registry trusts can do."""
    mode = ["--dry-run"] if dry_run else ["--trusted-publishing", "always"]
    return ["uv", "publish", *mode, str(artifact.path)]


class PypiPublish(Publish):
    """`publish` through uv."""

    def publish(self, product: Product, artifact: Artifact, dry_run: bool) -> None:
        subprocess.run(uv_publish(artifact, dry_run), cwd=artifact.path.parent, check=True)
