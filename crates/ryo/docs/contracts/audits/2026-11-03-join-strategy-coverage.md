| Authoritative dispatch | Actual semantic path | Refusal/behavior evidence (gate pending) |
|---|---|---|
| KernelStrategy::Existing + RepresentationKind::ALL | collected/indexed, sparse pairs, Selected shortcut, generic row | shared catalogue-driven representation cases; actual sparse path event |
| KernelStrategy::Column | fused selected-left/stored-right chain | shared forwarding/refusal probe; Column read count and eligible_chains; no optional fallback on error |
| TreeSteps::Lowered | private fragment SyntaxTree/SyntaxChild/Step RHS | same Positions refusal through production pipeline for all three RHS branches |
| TreeSteps::AsOperators | native/step inputs through mandatory row engine | same three pipeline cases use Values; Node/Span/Stretch finders only provide candidates, no output access |
| JoinInput / JoinRow exhaustive dispatch | Values/Positions and value/columnar left row | one shared probe exhaustively matches all variants; pure column-demand test |
| OperationKind::Join registry | JoinItem ordinary and explicit reach split | ordinary injected path; reach Some remains unchecked |
| Prepared DAG/cross-file/layout/schema branches | dependency/cache/admission/preparation only | source-linked occurrences map delegates to common executor; no independent ordinary Join emission |
| Non-Join and reach matches | no ordinary equality Join meaning | explicit excluded proof, not ownership credit |

Frozen at `65fc39b1` (a non-ancestor of `a85807c`); `core/src` and `operators/src` are byte-identical, and all 57 recorded sites were re-matched at `a85807c`. Full source occurrences and guard proofs: [sibling JSON](2026-11-03-join-strategy-coverage.json). Runtime status in this frozen audit is pending; no universal closure claim.
