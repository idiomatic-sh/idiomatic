SIM220/SIM221 preserve their original six output columns and four emit keys.
A four-column suppression projection goes through the unchanged 72ea helper.
Hidden IDs are derived by antijoin; a second antijoin filters the ORIGINAL full
candidate rows. The identity is `id`, matching the original suppression code.
There is no visible-ID join that could multiply rows.

58 unchanged rule cases / 116 actual 2ea worker records pass exact full-report
and pinned expectation comparisons, with exit zero and zero Unknown. Proof and
raw requests/stdout/stderr: `/tmp/suppress-extra-keys-20261101`. The shared replay
script accepts `--source-receipt extra-keys/source-receipt.json` for this batch.

Six focused adapter probes / 12 records preserve distinct extra-key rows that
share a finding ID and hence emit duplicate diagnostics at one location. Cases
cover two equal-shaped files, one-file-only suppression, correct/wrong inline
codes and block suppression. All reports exactly match the old full-row inline
implementation, with explicit nonempty counts (two duplicates/file), not merely
baseline/candidate equality. Raw evidence:
`/tmp/suppress-extra-keys-probes-20261101-v2`. This exercises two files but does
not assert public wire visibility of the provider's internal numeric node IDs.

An initial probe source incorrectly looked for names through semantic text and
produced zero seed rows; its required nonempty cardinality assertion rejected it.
That failed run remains `/tmp/suppress-extra-keys-probes-20261101`; it is not proof.
The corrected probe uses the existing identifier relation and never runs Ruff.
