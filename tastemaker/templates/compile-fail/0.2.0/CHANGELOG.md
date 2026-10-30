# compile-fail

## 0.2.0

The runner the `compiler-ui` rule 0.2.0 accepts: `cases.pass("tests/ui/pass_*.rs")`
then `cases.compile_fail("tests/ui/fail_*.rs")`, so the macro spec's
`tests/ui/pass_hostile_scope.rs` is compiled as a pass case. Moving up from 0.1.0: add the `pass` call
and rename each case `tests/ui/<case>.rs` to `tests/ui/fail_<case>.rs`. The runner opens with a
`//!` line that says what it runs, as the `missing_docs` lint asks of a test crate.

## 0.1.0

The first v2 release: the compile-fail runner: trybuild over every tests/ui case.
