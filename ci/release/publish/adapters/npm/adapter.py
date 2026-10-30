"""The `publish` step done through npm."""

from __future__ import annotations

import os
import subprocess

from ci.release.publish.interface import Publish
from ci.release.use_case import Artifact, Product


def npm_publish(artifact: Artifact, dry_run: bool) -> list[str]:
    """npm's publish of one tarball: its dry run, or an upload with provenance through npm's
    trusted publishing (OIDC), which only a workflow the registry trusts can do, or with `NODE_AUTH_TOKEN`."""
    mode = ["--dry-run"] if dry_run else ["--provenance"]
    return ["npm", "publish", "--access", "public", *mode, str(artifact.path)]


class NpmPublish(Publish):
    """`publish` through npm."""

    def publish(self, product: Product, artifact: Artifact, dry_run: bool) -> None:
        # npm reads a token from its configuration, not from NODE_AUTH_TOKEN: the token the workflow hands for a package's first
        # publish becomes that configuration; without it the upload goes through trusted publishing (OIDC).
        token = os.environ.get("NODE_AUTH_TOKEN")
        env = {**os.environ, "npm_config_//registry.npmjs.org/:_authToken": token} if token else None
        subprocess.run(npm_publish(artifact, dry_run), cwd=artifact.path.parent, env=env, check=True)
