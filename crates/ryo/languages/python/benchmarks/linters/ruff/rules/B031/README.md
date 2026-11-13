# B031 — repeated consumption of a groupby iterator

The query follows resolved `itertools.groupby` bindings and potential consumers of the second tuple target. It uses existing binding-use, phase and syntax facts; there is no provider or engine policy for this rule. Unknown callees receiving the iterator are potential consumers, not proof of consumption. The rule supplies diagnostics, not an automatic rewrite.

The complete [Ruff0.16.5 fixture](upstream.py) remains unchanged (237lines). All23 tagged findings remain; intended output adds the independently located `3136..3149` consumer after a comprehension-local shadow. The ordinary control bodies below run under `for _, g in groupby([1,1]):`; complete original programs and exact spans are in [cases.toml](cases.toml).

The actual shared worker passed all36 complete cases on both Python3.10 and3.13 targets, with full-report comparison. Each profile has twenty raw exact cases and sixteen explicit exact-source documented discrepancies. The shared offline collector independently accepts all36 with zero errors and zero contract-only cases. Focused peer review passed on the original source. The final `4bda76ec..a7a20029` singleton-prefix query and two negative controls received a separate narrow Fable PASS; the same two-target strict/offline proof passed on that exact final source. This is not an inventory promotion.

## Documented reference differences

Independent exact-target executions use CPython3.10.19 and3.13.5. Pinned Ruff0.16.5 ARM executable SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`, canonical target `python-target=3.10`, countertarget `python-target=3.13`. Counts below are complete finding multisets for unchanged originals, not dropped diagnostics.

| Original case | Ruff | Intended | Independently supported distinction |
| --- | ---: | ---: | --- |
| `upstream-full-fixture` | 23 | 24 | Comprehension-local `section_items` target; later consumer still sees outer grouper. |
| `reference-twice` | 1 | 0 | `a = g; b = g` only stores references. |
| `identity-twice` | 1 | 0 | `id(g); id(g)` observes identity. |
| `print-twice` | 1 | 0 | `print(g); print(g)` displays the grouper. |
| `observation-before-consumption` | 2 | 0 | `id(g); print(g); list(g)` consumes once. |
| `conditional-expression` | 1 | 0 | `list(g) if flag else list(g)` evaluates one arm. |
| `comprehension-shadow-does-not-replace` | 0 | 1 | `[g for g in range(2)]; list(g); list(g)` retains outer binding. |
| `nested-termination-stays-local` | 0 | 1 | `list(g); for ...: break; list(g)` inner break does not exit outer loop. |
| `iter-is-an-observation` | 1 | 0 | `iter(g); iter(g)` obtains the same iterator without advancing. |
| `next-is-incremental-iteration` | 1 | 0 | `next(g); next(g)` intentionally advances once per call; both return1. |
| `opposite-arm-replacement` | 0 | 1 | Only the opposite branch replaces `g`; the else branch consumes original twice. |
| `possible-replacement` | 0 | 1 | An optional replacement does not erase the unreplaced path. |
| `later-comprehension-iterator` | 0 | 1 | `[x for _ in [1,2] for x in g]` traverses `g` in each later clause. |
| `comprehension-filter-consumer` | 0 | 1 | `[x for x in [1,2] if list(g)]` consumes twice in its filter. |
| `generator-repeated-consumer` | 0 | 1 | `values = (list(g) for _ in [1,2]); list(values)` executes both consumers. |
| `one-inner-iteration-is-not-reuse` | 1 | 0 | `for _ in [0]: list(g)` has one consuming iteration. |

[Independent CPython3.10.19 evidence](../../profiles/B031/evidence/oracle.json) executes unchanged originals with both explicit `flag` values. Its only observation wrapper returns real builtin `list(value)` and logs grouper arguments; it does not supply a replacement iterator or branch. Companion assertions and the materialized scope example are labeled separately. [Raw initial receipts](../../profiles/B031/evidence/uncertified-cases.jsonl) preserve reference differences and earlier strict failures; reproduction scripts preserve the earlier source policy and tool/settings identities.

The initial draft incorrectly excused a never-called deferred body. That rationale was rejected: final function and generator cases positively diagnose repeated consumers within one execution phase. An enclosing unrelated loop does not repeat a fresh group's single consumer. A literal with at most one element and no unpacking does not create a second inner iteration; two consumers within its body remain reportable. Replacement is resolved by the existing binding-use facts, without a duplicate name/ordinal resolver.

Primary basis: [groupby iterator sharing](https://docs.python.org/3.10/library/itertools.html#itertools.groupby), [comprehension scope](https://docs.python.org/3.10/reference/expressions.html#displays-for-lists-sets-and-dictionaries), and ordinary conditional/loop/iterator execution. These are exact semantic and diagnostic-scope differences, not a claim that every upstream heuristic or unknown call is a proven bug. No upstream issue was opened.

The final prefix proof adds raw-exact negative controls for a singleton prefix before a later iterator or early filter. Only preceding potentially-many clauses multiply a later iterable; filters count their current prefix. The prior34-case receipts and peer-review source remain preserved separately.
