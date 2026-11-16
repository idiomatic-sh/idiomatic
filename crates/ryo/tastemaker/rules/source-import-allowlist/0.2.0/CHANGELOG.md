# source-import-allowlist

## 0.2.0

The source allowance equals its language's declaration, not a subset of it: a crate the root's `libraries` or `domain-libraries` declares and `src/tastemaker.toml` lacks is a finding, at the declaration (scenario `declared-crate-missing`). The rule takes no params: it reads both files, the root `tastemaker.toml` and its `src/tastemaker.toml`, paired by the language directory (scenario `languages-are-paired-by-directory`), so a placement includes both. It replaces the `provider-allowances` template's equality check, which language 0.4.13 no longer places. 0.1.0 is removed with this version: nothing has been released.
