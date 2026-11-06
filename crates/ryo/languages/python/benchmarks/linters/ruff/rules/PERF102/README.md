Pinned source: Ruff 0.16.5, tag commit `9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`; target Python 3.10, stable default settings. All prior expected arrays remain unchanged, including the formerly failing recursive-initializer examples and complete tagged upstream fixture.

Deferred for-loop checking includes async loops, nested tuple target unused checks, later shadowing uses and copied prior references, deletion/cleanup barriers, and positional-only call exclusion. Stable-default comprehension checking stays excluded because upstream gates it on preview. No dict-type requirement is introduced.

Validation: 33/33 exact cases pass on actual benchmark worker `65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b`. Each case has complete reference/candidate records and zero unknown rule/file pairs. This bounded evidence is not a whole-corpus parity certificate. Receipts/hashes: `assigned-type-20261101/verification.json`. No prepared-manifest registration changes.
