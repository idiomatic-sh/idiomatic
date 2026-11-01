# Read-time builtin and typing origins

The fragment accepts `checks: relation<check: node, root: node, file: file>` and returns matching `check,file` rows. It combines language binding-read facts, unique import identities and consistent qualified origins. Generic parameter shadows are constrained to their real owner and annotation/body scope. Unknown or conflicting reads are withheld.

A call resolves its callee before evaluating arguments. Unrelated imports do not change this rule. No seen-module flags, linter worklist snapshots or forward-module preference participate in this helper. The historical name remains for import compatibility; `mode="pattern"` enables qualified-path pattern matching, while other modes use the same read-time origin.

`binding-type` classifies actual parameter annotations and proven initializers conservatively. A function object's return annotation does not classify the function object as its return value. This change intentionally differs from retained pinned-oracle outputs for such objects. Actual call-result inference is outside this correction.

See `evidence/independent-read-origin` for exact worker identity, retained reference reports, source snapshots and independently intended expectations. Round-rule drivers are parked experiments, not newly certified rules. Generic-syntax controls targeting Python3.10 are scope guards, not admitted oracle discrepancies.
