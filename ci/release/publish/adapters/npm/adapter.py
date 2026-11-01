"""The `publish` step done through npm."""

from __future__ import annotations

import subprocess

from ci.release.publish.interface import Publish
from ci.release.use_case import Artifact, Product


def npm_publish(artifact: Artifact, dry_run: bool) -> list[str]:
    """npm's publish of one tarball: its dry run, or an upload with provenance through npm's
    trusted publishing (OIDC), which only a workflow the registry trusts can do."""
    mode = ["--dry-run"] if dry_run else ["--provenance"]
    return ["npm", "publish", "--access", "public", *mode, str(artifact.path)]


class NpmPublish(Publish):
    """`publish` through npm."""

    def publish(self, product: Product, artifact: Artifact, dry_run: bool) -> None:
        subprocess.run(npm_publish(artifact, dry_run), cwd=artifact.path.parent, check=True)
