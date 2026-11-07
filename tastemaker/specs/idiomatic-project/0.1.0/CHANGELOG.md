# idiomatic-project

## 0.1.0

The monorepo-wide rules a product places once: file-length, catch-all-names, no-registry-renames, workspace-membership, workspace-completeness and no-tsv, which was new. A rules-only spec: it has no template, so it owns nothing and its rules reach the parts placed inside it.

`file-length` leaves out `api.txt`, the generated record of a crate's public API.

Rule added: `no-machine-paths-shipped`, at `error`: no shipped file (docs, data files, comments) names a path under `/workspaces`, `/tmp`, `/home` or `/Users`; tests, benches, fixtures and the locks are left out.

Edited in place: `no-tracker-text-shipped` places the process phrases `the owner`, `owner's decision`, `the spike`, `the devcontainer` and `an agent` beside the issue ids.
