# memory-bench

## 0.1.0

The memory bench leaves the `distribution` spec for a spec of its own, placed by tastemaker at `cli/benches/memory`: the bench cut to one mode (the peak of a cold check, and `validate` cold and warm, over one target), its proofs run before it measures and alone without `--bench`, the instrument's probe written through the child writer (`executable.rs`), the install by Cargo's install, and the Ruff target beside it with its policy. Its `measure`, `resources` and `instrument` templates are distribution 0.3.5's `bench-measure`, `bench-resources` (the `/proc` sampler) and `bench-instrument`, with the timing rounds, the edit and the registries gone.

### Moving up from distribution 0.3.5

- Move `distribution/benches/{measure,resources,instrument}.rs` to `cli/benches/memory/`, `distribution/target_test.rs` to `cli/benches/memory/project.rs`, `distribution/proof/{preparation,summary}_test.rs` to `cli/benches/memory/project/{preparation,summary}.rs` and the contract proofs to `cli/benches/memory/proof.rs`.
- Move the cargo registry's target to `cli/benches/memory/<project>/` (no `edit.patch` or `edit-findings.toml`) and its policy to `<project>/policy/`.
- Declare `[[bench]] memory` at `benches/memory/main.rs` (crate 0.14.0 renders it).
