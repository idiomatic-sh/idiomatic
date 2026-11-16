# nextest-config

## 0.2.0

The content adds the `ci` profile, `[profile.ci]` with `fail-fast = false`, under a comment naming `NEXTEST_PROFILE=ci`, which check.yml sets: a CI machine runs every test, so one red does not hide the rest of its reds. A file without the block is a `nextest-config#template` finding (scenario `ci-profile-missing`). The `extra-content` scenario's extra is a `[profile.ci.junit]` table, since 0.1.0's extra is now the content.

### Moving up from 0.1.0

Append the comment line, `[profile.ci]` and `fail-fast = false` to `.config/nextest.toml`.

## 0.1.0

The first v2 release places a product's `.config/nextest.toml`. Its default filter excludes bench targets and cucumber BDD runners; `cargo test --all-features --benches --test '*bdd'` runs those targets, every feature on so a target that needs one is built.
