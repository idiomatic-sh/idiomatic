# asymptote-config

## 0.9.0

`test-doc` runs the doctests on the nightly toolchain the product's `Cargo.toml` pins in `[workspace.metadata.api] nightly`,
read when the tool runs, so the version lives only in that manifest. Stable rustdoc does not check the error code of a
`compile_fail,E0…` doctest: such a doctest passed for any error, so a field renamed and made `pub` left its E0616 proof
green. A product that takes 0.9.0 declares that key. Nothing else changes.

## 0.8.0

The clippy tools run `-- --cap-lints warn` and pipe through `cargo_diagnostics.py` with `python3`: a denied lint is a SARIF
result the lock judges instead of aborting the run, only the codes the workspace lint tables deny (in workspace files) are
forwarded, and a compile error still fails the run. The default-features tool goes through the same filter. Every scenario
carries the same run lines.
