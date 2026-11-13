# UP035/UP036 full target-version fixture proof

Both canonical default-target rules pass their complete 38-case suites.
UP035 also passes the same 38-case breadth at Python 3.7, 3.8, 3.9, 3.11, 3.12,
3.13, and 3.14. UP036 passes its full 38 cases at Python 3.7 and 3.13 as well
as the default 3.10. No cases, settings, full upstream fixtures, or explicit
reference discrepancies were dropped to obtain these results.

Across the target suites, 418 intended case pairs pass: 415 raw exact and
three independently documented UP036 micro-version discrepancies. In the
canonical default-target summary, 76 intended case pairs pass: 75 raw exact
and one documented discrepancy. These are separate singleton executions,
not a new combined full-inventory run or performance evidence.

`summary.json` holds the two default-target rows. `controls.json` binds all
11 executed target suites to their exact source queries, full cases, settings,
profiles, native per-case receipts, registrations, and collector summaries.
The generator validates every candidate/reference case pair through the
existing receipt policy, checks every counter-target, validates the current
UP036 evidence certificates, and rejects changed sources or earlier summaries.

Worker SHA256:
`e0092188f1af6786f4981680a7129445f33117050343388ba9cb69016891266a`
Collector SHA256:
`813f50ad3cf909b23a48da6801a50d756c349ab47129e7dd07815c6e277dd004`
Pinned Ruff SHA256:
`52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`

`raw-receipts.tar.gz` preserves the initial grammar failures, UP036's first
17/38 candidate result, all subsequent successful receipts, and sealed input
stages byte-for-byte. `provenance.json` explicitly distinguishes original
source batches from the corrections actually executed. Early registrations'
original-patch provenance field is not presented as the executed corrected
source. Final query hashes and the original/correction/final patches bind the
actual bytes independently. Default summary instances add canonical settings
source metadata only after exact execution hashes/settings match the existing
registration function; original collector receipts remain unchanged.

The UP036 exception is limited to exact supplied fixtures. The micro component
can vary for supported patch releases with the same major/minor minimum;
independent Python execution of explicitly modeled patch-version values takes
both branches. Those modeled values are clearly distinguished from the real
host interpreter version. Pinned Ruff's original findings remain separately
expected and recorded. Standard version and major-component controls remain
positive. No blanket waiver or upstream bug emulation was introduced.

The final queries additionally correct two authoring defects exposed by the
real worker: unsupported source comments/negated numeric predicates, and an
incorrect assumption that physical child ordinals were field-local indexes.
The source/readiness history is retained rather than relabeled as passing.

For reproduction, use canonical full-inventory registration and the same
explicit target settings; the archive's sealed input profile paths are listed
in controls.json. Run the supported shared collector `verify-inventory`
interface with the registered sealed worker command. No Rust build was run
for these replays. The generated inventory is 377 PASS / 36 FAIL / 413 defaults
only when all earlier and new source-bound evidence remains valid.
