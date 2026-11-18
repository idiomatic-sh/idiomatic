# distribution-wiring 0.2.0

The distribution is its own unpublished crate (distribution 0.4.0), so the wiring is the workspace's, not the command line crate's: a product whose `distribution/Cargo.toml` exists lists `distribution` in its workspace's `members`, and the crate says `publish = false`. The command line crate's gated `[[test]] distribution`, its sibling path and its `distribution` feature are gone, and so is the check of them.

## Moving up from 0.1.0

- Add `"distribution"` to the workspace's `members` and `publish = false` to `distribution/Cargo.toml`.
- Delete the command line crate's `[[test]] distribution`, `[[bench]] distribution` and `distribution` feature.
