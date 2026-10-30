# compiler-ui

## 0.2.0

The runner runs two case sets: the pass cases `tests/ui/pass_*.rs`, which
must compile, and the compile-fail cases `tests/ui/fail_*.rs`. The macro spec 0.2.0 places
a required `tests/ui/pass_hostile_scope.rs`, and 0.1.0's single `tests/ui/*.rs` glob would
run it as a compile-fail case. `fn ui` holds exactly the `TestCases` binding, one
`cases.pass("tests/ui/pass_*.rs")` and one `cases.compile_fail("tests/ui/fail_*.rs")`:

```rust
#[test]
fn ui() {
 let cases =::trybuild::TestCases::new();
 cases.pass("tests/ui/pass_*.rs");
 cases.compile_fail("tests/ui/fail_*.rs");
}
```

The query is `arch.compiler-ui` 0.1.0's two-call query with its globs changed and its
path filters reading their relations directly, as in 0.1.0. The fixtures are
`arch.compiler-ui` 0.1.0's in the v2 format with the new globs; `one-glob` (0.1.0's shape)
is new and fails.

### Moving up from 0.1.0

- Rename each compile-fail case `tests/ui/<case>.rs` (and its `.stderr`) to
 `tests/ui/fail_<case>.rs`.
- Add `cases.pass("tests/ui/pass_*.rs");` before the `compile_fail` call and change its
 glob to `"tests/ui/fail_*.rs"`.

## 0.1.0

The v2 release of `arch.compiler-ui` 0.1.0, under its unprefixed name: one
`cases.compile_fail("tests/ui/*.rs")`.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.
