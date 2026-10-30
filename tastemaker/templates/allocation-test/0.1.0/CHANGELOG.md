# allocation-test

## 0.1.0

The shared allocation test, moved out of the `cli` spec's `cli-allocation-test`: the `#[global_allocator]` static with `dhat` and the `main` that runs the profile, placed by the `cli` spec at `tests/blocks.rs` and by the `kernel-crate` spec at `tests/allocation.rs` (the language crate's dhat allocation test). Fixtures: `example`, `extra-content` (a kernel crate's real shape), `missing` and `missing-allocator`.

The content is the kernel language crate's allocation test in full, so a scaffolded test counts
the blocks of a run and is not a bare `main`.
