# idiomatic-project

## 0.3.0

`no-tracker-text-shipped` no longer reports every `the owner's`: the owner of a module (`the owner's own graph tables`) is the product's own word. It reports the phrases that name who decided: `the owner's figure`, `number`, `call`, `choice`, `ruling`, `word`, `request`, `budget`, `target` and `bar`, with `owner's decision` as before. Everything else is as in 0.2.0, which stays as published.

## 0.2.0

`file-length` leaves out the data of a linter benchmark (json, jsonl, toml, ryoql, txt and md beneath `benchmarks/linters/*`): the preparation manifest binds it by hash, so it is evidence, not authored source. A long `.rs` or `.py` file there is still reported. Everything else is as in 0.1.0, which stays as published.

## 0.1.0

The monorepo-wide rules a product places once: file-length, catch-all-names, no-registry-renames, workspace-membership, workspace-completeness and no-tsv, which was new. A rules-only spec: it has no template, so it owns nothing and its rules reach the parts placed inside it.

`file-length` leaves out `api.txt`, the generated record of a crate's public API.

Rule added: `no-machine-paths-shipped`, at `error`: no shipped file (docs, data files, comments) names a path under `/workspaces`, `/tmp`, `/home` or `/Users`; tests, benches, fixtures and the locks are left out.

Edited in place: `no-tracker-text-shipped` places the process phrases `the owner`, `owner's decision`, `the spike`, `the devcontainer` and `an agent` beside the issue ids.
