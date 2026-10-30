# asymptote-workspace

## 0.1.0

`ci-tests` runs in the `check` group, the landing gate, beside the products' tests (`asymptote-config` 0.6.0's groups: lint, check, release and perf).

`tastemaker-validate` is gone, and with it the scenario `no-tastemaker-validate`: `tm check` proves every definition first, as `tm validate` does.

`shellcheck` checks every tracked shell script in the lint group; the scenario `no-shellcheck` shows its absence is a finding.

Edited in place: every `run` and `fix` names a bare command on the PATH (see `asymptote-config`); `merge-conflict` reads the working tree with
`git grep --untracked`, so a file git does not track yet cannot pass and be staged unchanged, and the cache of a pass is no stale one; the `tastemaker` tools
read `asymptote.toml`, which `tastemaker.toml` governs; `commitizen` has no `reuse` key, because the cache stamp of a tool whose command names a git range
holds the commits it resolves to; `ci-tests` reads the packaging pins of `crates/*/distribution`. A scenario shows the refusal of a version or a launcher in a `run` line.

The repository root's `asymptote.toml`: the workspace. `members` names the products (`crates/*`, each a directory with an `asymptote.toml` of its own, placed by `asymptote-config`), and the root's own tools are the checks no product owns: the Tastemaker check of the root's `tastemaker.toml`, taplo for TOML, yamllint for YAML, editorconfig-checker for final newlines and trailing whitespace, `git diff --check` for conflict markers, typos, actionlint, and the CI jobs' own tests. `asymptote check --groups lint` at the root is the git hook, and `asymptote check` is CI.
