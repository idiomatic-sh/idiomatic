# benchmark

## 0.2.3

Places the linter adapters at `adapter@0.4.0`, so a product locks one version of the adapter spec with the ports that place it. Everything else is as in 0.2.2.

## 0.2.2

Places `benchmark-linter` 0.2.2, which places the linter's own Cargo package. Nothing else changes.

Edited in place: places `adapter@0.3.2` for the linter adapters, where it named their earlier versions, so a product that places it locks one version of each rule: two versions of one rule share its id, and the earlier entries ran the later query without their params (a `test-shape@0.1.0` entry reported nothing).

## 0.2.1

Places `benchmark-linter` 0.2.1, which places the parts of the sharing fixtures. Nothing else changes.

## 0.2.0

Places `benchmark-linter` 0.2.0, which admits `ryo_benchmark_contracts` and the language's settings in composition. 0.1.0 stays as published.
