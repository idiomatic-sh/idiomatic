# idiomatic-project

## 0.3.3

Places the two carve-out rules on every `tastemaker.toml` of the product, beyond its test inputs and Tastemaker's examples: `manifest-placements@0.1.0` (no one-file placement but a config role; entries and `closed = false` only in the workspace manifest or a manifest a role places, a language's own) and `manifest-excludes@0.1.0` (every exclude and allowlist path lies in a role; a test contract starts at the product's public API), with the role sets the cleanup settled. Scenarios `a-one-file-placement-in-a-side-manifest-is-reported` and `an-exclude-of-a-crate-is-reported`. Everything else is as in 0.3.2.

Edited in place: `manifest-excludes` takes no `rule-roles`; `key-roles` pairs `allowed-paths` with `**/constants.rs`, `**/errors.rs` and `**/schemas.rs`, the files whose literals that allowlist admits, now that those globs are no shared role.

## 0.3.2

`no-tracker-text-shipped` also reports a tracker id with no digit and no child number when it stands in parentheses (`(ryo-abcd)`, `(ryo-abcd census)`) or after "bead" (`bead ryo-abcd`, `bead_ryo-abcd`, `bead-ryo-abcd`): agents write ids that way, and a bare digitless id stays out because it reads like a package name (`idiomatic-sh-ryo-sdk`). Everything else is as in 0.3.1.

## 0.3.1

`crate-dependency-depth` allows a package to reach fourteen local packages, not nine, and only the ryo workspace places this version: ryo's decomposition of its engine into a stack of crates (the domain, the store and the runtime below execution) adds internal packages behind the sdk, and every language provider reaches the packages the sdk reaches plus the sdk. Everything else is as in 0.3.0, which stays as published.
