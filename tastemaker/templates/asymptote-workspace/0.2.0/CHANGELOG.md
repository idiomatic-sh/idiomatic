# asymptote-workspace

## 0.2.0

The root's tools move to root-only groups, which no product has: `repository` (tastemaker, taplo, yamllint, editorconfig, merge-conflict,
typos, shellcheck, actionlint and commitizen) and `repository-check` (ci-tests). A check at the root of a group runs every member's tools of
that group too, so the old `lint` and `check` groups ran every product's lint and test tools from the root job; now
`asymptote check --root . --groups repository,repository-check` runs the repository's checks alone, and each product's `lint` group runs in
that product's own job. The pre-commit hook runs `lint` and `repository`. Scenario ids are unchanged.

`ci-tests` names the whole tree (`inputs = ["."]`): its suite reads the workflows, every product's manifests, `.gitleaks.toml`,
`.gitleaksignore`, `cz.toml` and cargo's metadata of every workspace, and a persisted store reuses it while its inputs do not move.
`commitizen` checks the branch's commits alone. The pull request's title, which it read from `$GITHUB_EVENT_PATH`, is no
file and no stamp holds it, so a stored pass outlived an edited title: a repository checks it in a group no job keeps a store
for. Edited in place: 0.2.0 is unreleased.

## 0.1.0

`ci-tests` runs in the `check` group, the landing gate, beside the products' tests (`asymptote-config` 0.6.0's groups: lint, check, release and perf).

`tastemaker-validate` is gone, and with it the scenario `no-tastemaker-validate`: `tm check` proves every definition first, as `tm validate` does.

`shellcheck` checks every tracked shell script in the lint group; the scenario `no-shellcheck` shows its absence is a finding.

Edited in place: every `run` and `fix` names a bare command on the PATH (see `asymptote-config`); `merge-conflict` reads the working tree with
`git grep --untracked`, so a file git does not track yet cannot pass and be staged unchanged, and the cache of a pass is no stale one; the `tastemaker` tools
read `asymptote.toml`, which `tastemaker.toml` governs; `commitizen` has no `reuse` key, because the cache stamp of a tool whose command names a git range
holds the commits it resolves to, and on GitHub Actions, run for a pull request, it checks the pull request's title too, which a squash merge makes
the commit main keeps; `ci-tests` reads the packaging pins of `crates/*/distribution`. A scenario shows the refusal of a version or a launcher in a `run` line.

The repository root's `asymptote.toml`: the workspace. `members` names the products (`crates/*`, each a directory with an `asymptote.toml` of its own, placed by `asymptote-config`), and the root's own tools are the checks no product owns: the Tastemaker check of the root's `tastemaker.toml`, taplo for TOML, yamllint for YAML, editorconfig-checker for final newlines and trailing whitespace, `git diff --check` for conflict markers, typos, actionlint, and the CI jobs' own tests. `asymptote check --groups lint` at the root is the git hook, and `asymptote check` is CI.
