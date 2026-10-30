# nextest-config

## 0.1.0

The first v2 release places a product's `.config/nextest.toml`. Its default filter excludes bench targets and cucumber BDD runners; `cargo test --all-features --benches --test '*bdd'` runs those targets, every feature on so a target that needs one is built.
