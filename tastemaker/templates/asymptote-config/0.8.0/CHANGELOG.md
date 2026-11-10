# asymptote-config

## 0.8.0

The clippy tools run `-- --cap-lints warn` and pipe through `cargo_diagnostics.py` with `python3`: a denied lint is a SARIF
result the lock judges instead of aborting the run, only the codes the workspace lint tables deny (in workspace files) are
forwarded, and a compile error still fails the run. The default-features tool goes through the same filter. Every scenario
carries the same run lines.
