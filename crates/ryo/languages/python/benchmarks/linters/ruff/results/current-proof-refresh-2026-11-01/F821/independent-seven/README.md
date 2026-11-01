# Exact independent evidence for seven F821 differences

The inputs are copied by exact name from the retained F821 reproduction. Pair indices are zero-based in its authoritative cases.jsonl; they are not TOML declaration order. inputs.json binds original207-pair records and each exact source byte string. manifest.json binds the probe, inputs, interpreters and outputs.

Each installed interpreter runs probe.py with -I. The script calls compile(source, filename, "exec", dont_inherit=True) only; returned code objects are never executed. All four interpreters were already installed. No interpreter was downloaded or built, no Rust was compiled and no Ruff oracle was rerun.

All six nested-future cases reject in3.10.19/3.13.5/3.14.0/3.15.0rc2 at the forbidden import. Exact3.10 error locations match Ryo invalid-syntax ranges. This resolves syntax validity, not the additional undefined-name recovery diagnostics: invalid source has no executable runtime meaning, and compile() does not specify a linter recovery policy. Those F821 differences remain explicitly unadjudicated.

The exact relative-future case rejects in3.10.19, accepts in3.13.5, and rejects in3.14.0/3.15.0rc2. Three disambiguating controls show3.10 enables the annotations future flag for an ordinary relative signature, while3.13+ do not.3.14+ reject the plain walrus signature too, under native annotation rules. Pinned Ruff therefore agrees with the exact3.10 implementation for this case; this is not proof of a universal oracle defect. The documented absolute future-statement contract differs from observed3.10 activation. Explicit intended-contract review remains necessary; do not change the generic provider to mimic incidental implementation behavior.

classification.json records conclusions, source hashes and limits. Earlier order-reference-receipts.json links were Ruff oracle observations, not independent Python evidence; this bundle supplies the actual independent observations. It does not admit discrepancies, change rules, rewrite expected arrays or alter the original207 pairs /32 differences.
