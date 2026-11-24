# Pricing two smaller Tastemaker queries

This is a trial against the pinned Tastemaker inputs. The published pack texts are untouched. The base for the paired whole-session runs is the published integration at `caf71614a`. The independent rule-level observer is `ec4ab9d31`, which adds the ordered file-path screen to that integration. All measurements outside the coordinator's window are loaded. The quiet reference remains **17.293 CPU-s** for Ryo in the real check at `197b4b614`.

## Fixtures only in tests

The rule reports production references to declarations that exist only in test or fixture builds. Its method, typed-path and package-local name branches have different sealing rules. The ending excludes test paths and uses within gated items.

The original attaches every qualifying identifier to its name-reference node, then joins raw syntax edges with kind tables repeatedly to identify method calls, ordinary path segments, capitalized qualifier paths and the segments they qualify. These joins carry the identifier and both ends of the edge through intermediate tables before the final name seal can be tested. The largest original select delivered **3,210,547** child rows to build **103,030** rows. Reported in the file-phase census, most contribution rows are never consumed by the closed phase.

The rewrite uses native child or parent navigation for the fixed edges. The method branch takes only method declarations into its enclosing-gate walk. The segment-to-name-reference relation serves both path branches. The test for an uppercase qualifier runs on the identifier side before it joins the qualifier references. The new qualifier paths require their path and its qualifier both to be Rust path nodes. Rust's syntax tree represents a qualified path as `PATH(PATH, PATH_SEGMENT)`, and a `PATH_SEGMENT` has a `PATH` parent, so the added kind guards are entailed on this reader. The walk keeps the same node IDs and projects the same edge endpoints. Tokens must still be direct children of a name-reference node. All gates, the file-package keys, the sealed-name antijoins, source spans and final projection are preserved.

## Field written never read

The rule reports a private struct field that is written and has no credited read of the same spelling in its crate. A nested variant or union field is excluded by taking the nearest owning kind before testing for a struct. The query exempts fields using PhantomData and structs whose derive attributes name a nontrivial derive. Reads include field accesses, selected pattern names and direct tokens in macro token trees.

The original joins all nonempty syntax-token text to all immediate edges, then keeps only identifiers. The filter observes **4,964,597** inputs and makes **1,190,028** outputs in the largest cohort. Its subsequent parent lookup joins against all tree nodes. The rule also carries duplicate file-and-field witnesses into multiple path captures, although closed execution only tests whether a crate-and-field key is present.

The rewrite asks for nonempty identifiers first, visits their parent using the typed axis, and projects the endpoints and text the later branches need. The parent domain still enforces empty-text tree nodes. It filters nontrivial derive names before joining to the derive-attribute IDs, then follows the direct edge to a struct. The nearest-owner domain still contains all three kinds, preserving the shadowing of an outer struct by a nearer variant or union. Finally, identical `(file, field)` rows are projected once before deterministic path capture. Their emitted existence at `(crate, field)` is independent of multiplicity. Declarations and diagnostic spans are carried separately, so the rewrite cannot collapse multiple reported declarations at distinct locations. Both captured parameter values remain bound at runtime. The offline plan snapshots specialize those two values because the `vec lower` command does not bind caller parameters.

## General lowering opportunity

Fuse a selection and projection into its producer whenever a predicate's column origins belong to one input of a deterministic inner join. In particular, rewrite an equality between one endpoint of a declared tree-edge relation and a typed node ID into traversal from the node, with the other endpoint's local predicates as its domain. A predicate referring only to the child side can become a native view test before the edge lookup. Keep the edge orientation, column identities, both endpoints when consumers read both, and any additional residual predicates. Validate the reader's declared hierarchy and endpoint schema before using an axis. Preserve absence and refusal, and decline the fold for bag-sensitive results, order-sensitive operators or ambiguous provenance unless their equivalence is explicit.

The two trials expose these transformations without adding an engine operator. The lowering should also recognize when every downstream use of a producer is an existence test on a complete key and narrow its projection there. That is a separate rule because dropping multiplicity is valid only with proof of every consumer. The fixture seal additionally travels across the closed boundary. A local tree rewrite alone cannot push that cross-file demand back to the producer.

The same optimizer can run over Tastemaker and linter plans. The other eighteen queries need a structural eligibility census before assigning savings. The twenty-rule 3.93-second figure is gross work, and these two trials do not price the remainder.

The source census found 154 named syntax-edge joins without a direct edge ordinal or field read across **12 of the other 18** texts. Those twelve carry **2.017 gross quiet-scale file CPU-s**. This is structural eligibility for inspection, since later consumers, physical-plan folding and cost can still rule out an optimization. For example, the census finds six such joins in no-env-reads and 22 in test-shape. The remaining six include declarations-defines, which already takes a single native source. [All eighteen](results/other-eighteen.tsv).

## Correctness and observed rule costs

For each rewrite, the forward and reverse Ryo capture receipts are byte-equal to the pinned control for all 46,831 requests. The edited text is exercised by 14 fixture-rule requests and 37 field-rule requests. Root, Tastemaker and Asymptote preserve all 36,279, 36,315 and 36,271 receipts respectively. Both rewrites pass their own frozen case in each order and all 294 cases. The private live overlays ran the original host check with 46,831 answered requests, zero failed calls and byte-equal released SARIF. [Gate ledger](results/correctness.tsv).

The map observer on `ec4ab9d31` records byte-equal typed receipts and real SARIF. It uses the same image for both texts and gives separate passes to CPU and row counts. CPU is loaded and charged to each rule's file evaluations, across an observer series that spans a one-minute load of 20.18–27.10. These observations are single-arm samples for attribution. The whole-session comparisons use the separate clean image and three alternating pairs.

| Rule and workload | File evaluations | Select child rows | Select output rows | Loaded rule CPU-s |
|---|---:|---:|---:|---:|
| Fixtures, replay | 7,830 → 7,830 | 17,493,182 → 17,445,653 | 11,526,003 → 10,150,876 | 1.209 → 1.310 |
| Fixtures, real check | 7,830 → 7,830 | 17,493,434 → 17,445,731 | 11,526,127 → 10,150,914 | 1.264 → 1.485 |
| Field, replay | 4,871 → 4,871 | 10,380,241 → 5,774,169 | 9,112,883 → 4,521,610 | 0.670 → 0.502 |
| Field, real check | 4,887 → 4,887 | 10,381,666 → 5,774,193 | 9,112,843 → 4,521,629 | 0.668 → 0.512 |

All observed evaluations for these rules supply a closed contribution. The productive counts stay at 7,830 for fixtures and 4,238 for fields on both workloads. A shape's child-row count covers rows delivered to selects. When the text exchanges a select for an axis, work performed inside that axis is outside this count. Fixtures did 12% less select-output work, with a 0.3% reduction in its select inputs and higher observed rule CPU. Its largely unchanged input count and new axes explain why the smaller output is insufficient evidence for a faster physical plan. The field rewrite discharged 44% of the select input and 50% of the select output. This is the stronger case for predicate pushdown because the work omitted is construction of unrelated identifiers' sibling tokens before filtering. [Rule-level counts and loads](results/per-rule.tsv).

## Clean whole-session comparison

Each row is three alternating one-worker pairs on `caf71614a`. The CPU, wall and peak columns give separate arm medians. The paired column gives candidate minus control for every pair, followed by its median. Replay CPU covers the whole replay process. Real-check CPU in the first column covers Ryo.

| Workload and candidate | Loaded CPU-s | Signed CPU-s by pair (median) | Whole command CPU-s | Wall s | Peak MiB | One-minute load |
|---|---:|---|---:|---:|---:|---:|
| Replay, fixtures | 24.820 → 23.660 | +0.590, −0.140, −1.390 (−0.140) | 24.820 → 23.660 | 25.791 → 24.094 | 1,100.33 → 1,093.97 | 23.11–27.33 |
| Real check, fixtures | 22.145 → 22.076 | +0.082, −0.069, −0.257 (−0.069) | 30.646 → 30.667 | 31.939 → 30.686 | 1,666.11 → 1,660.60 | 18.05–20.28 |
| Replay, field | 21.010 → 21.440 | +0.920, +0.430, −0.380 (+0.430) | 21.010 → 21.440 | 21.179 → 22.312 | 1,108.00 → 1,098.74 | 20.18–23.58 |
| Real check, field | 22.616 → 22.194 | −0.223, −0.604, −0.289 (−0.289) | 31.360 → 30.936 | 31.362 → 31.090 | 1,657.16 → 1,667.00 | 12.89–18.05 |

Real-check second-half paired medians were −0.001 CPU-s for fixtures and −0.117 for the field rule. Replay second-half paired medians were −0.369 and +0.209 respectively. Both trials evaluated 304,764 file-program pairs on replay and 310,545 live in every clean arm. The field trial had three faster live checks, two slower replays and a 9.84 MiB higher live median peak. The fixtures rule's measured local CPU went up despite a small favorable whole-check median. The package and engine diff is empty. [All phase pairs](results/pairs.tsv) · [Lowered plan shapes](results/plan-shapes.tsv).
