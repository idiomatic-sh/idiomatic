# lint-allocation-test

## 0.1.0

The allocation row of the whole per-file path: one test file, `tests/allocation.rs` of the core, run without a harness so it is alone in its process, with `dhat` as the global allocator, linting a fixed corpus twice and counting the blocks each file's job allocates against what the parser library allocates on its own.
