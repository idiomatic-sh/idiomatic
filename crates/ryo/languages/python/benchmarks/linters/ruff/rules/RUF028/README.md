# RUF028 formatter directive placement

Candidate implementation: 97 full-report cases and 67 intended diagnostic spans,
including the exact pinned fixture and its 11 snapshot findings. Sealed-worker
replay matched all 97 intended cases; admission is still pending. TargetPython3.10 and `accepted-syntax` applicability are
explicit; the phase comes from the canonical0.16.5 registry.

The query recognizes exact `fmt: off/on` and `yapf: disable/enable` commands with
Python lexical whitespace. `fmt: skip` can occupy a `#`-delimited comment segment.
Off/on commands with extra explanatory text, non-Python whitespace or different
capitalization are ordinary comments. Directives need not balance: this rule
checks attachment and placement, rather than silently imposing a new pairing
policy. Diagnostic spans are the real entire comment token spans.

Skip must trail code. On must occupy its own line. No directive can suppress an
expression, pattern, parameter or argument-list interior. Statement, handler,
case, decorator and clause boundaries can own a directive. Off/on between a
completed decorator and the definition name is invalid unless the comment really
attaches as a more-indented trailing decorator comment. Off directly before an
alternate body is invalid when the preceding comment group dedents below the
preceding statement's physical indentation or the comment aligns with or
dedents from the alternate owner. The owner-column check handles nested
`else` gaps whose lossless-tree comment owner is the enclosing scope.

Attachment uses the native lossless tree, not merely the smallest enclosing
semantic range. Each AST branch starts at most one following gap of sibling
leaves. `ordered-dfs` walks those real leaf identities and stops at the next AST
branch; gaps do not overlap. Root leaves are included explicitly, including a
single final EOF comment. This finds preceding/following native nodes without
constructing all token pairs, transitive sibling closures or fabricated IDs.
Three bad-prefix modes (intervening code, non-deeper indentation and shallower
indentation) use a second segmented leaf walk. Roots start immediately after a
real barrier; the next barrier or AST branch stops that segment. Segments do not
overlap within a mode, avoiding comment-prefix or token-pair cross products.
Decorators are matched by their actual immediately preceding AST gap, rather
than joining every decorator to every comment in a definition. These are
structural bounds, not measured performance results.

Identifiers are skipped as formatter attachment anchors; their real spans still
count as intervening code. A comment trails a preceding node only when no code
intervenes and, on an own line, every preceding own-line comment in that gap is
more deeply indented. Ordinary comments participate in that indentation group.

The first alternate body is identified by native clause/handler siblings or the
actual `else`/`finally` keyword in the gap. There is no handwritten fixed-depth
walk or simulated linter binding history. Source-order and indentation are part
of formatter placement policy; none is exported as a Python runtime fact.

The independent contract uses physical line indentation for semicolon-separated
statements. The pinned implementation's `indentation_at_offset` fallback can use
zero when code precedes a node on its line; the semicolon control deliberately
exposes that distinction for the canonical run instead of copying the fallback.
Parenthesized-atom, deeper decorator attachment, later alternate clauses, inline
ordinary comments and quoted annotations are retained as nontrivial controls;
unexpected results must remain raw differences until investigated. No presumed
reference discrepancies are certified here.

The97 cases cover directive spellings and matching segments, own/trailing
placement, expression and pattern interiors, function signatures, decorator
attachment, first/later alternate bodies, ordinary-comment group order, deeper
trailing comments, tabs/form feed, Unicode byte spans, CRLF, EOF, strings,
file/inline suppression and two independent files. The sealed Python worker SHA256
`8b09a4771af76452dfde69252c501f7a9a71e6eb8827503aafaf2b60bd68bedd`
reported 97/97 candidate cases, including all 11 pinned-fixture findings. Ruff
reported 93/97 intended cases; four source-bound differences remain without
certified reference exceptions. Bounded-size memory/CPU and integration admission
remain pending.

Research pin: Ruff0.16.5 commit
`9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`. Primary behavioral sources:

- `crates/ruff_linter/src/rules/ruff/rules/invalid_formatter_suppression_comment.rs`
- `crates/ruff_linter/src/rules/ruff/rules/suppression_comment_visitor.rs`
- `crates/ruff_python_trivia/src/comments.rs`
- `crates/ruff_python_ast/src/helpers.rs` (`comment_indentation_after`)
- `crates/ruff_python_ast/src/node.rs` (alternate-body classification)

The tagged fixture/snapshot hashes are in `provenance.json`. This is an
independent relational implementation of the observable placement policy, not a
copy of the reference visitor architecture.

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/RUF028/benchmark.toml)). **Scope:** Authored formatter-placement policy difference; no upstream bug claim.

The 4 certified cases below retain their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `directive-whitespace`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 /home/vscode/.local/bin/ruff check --isolated --output-format json --cache-dir ./ruff-cache --select RUF028 --target-version py310 -- case.py
```

### Directive whitespace

Cases (1): `directive-whitespace`.

Representative exact source, `directive-whitespace` (`case.py`): JSON-escaped to preserve whitespace:

```json
"\t#\tfmt:\tskip\t\n"
```

- Intended: RUF028 case.py:1–13. Pinned Ruff observed: none.
- Basis: The authored directive grammar admits Python horizontal tab whitespace around fmt: skip; CPython tokenization confirms the exact own-line comment. Primary evidence: [cpython310-observation.json](../../profiles/RUF028/evidence/cpython310-observation.json).
- Reproduce: use the [profile](../../profiles/RUF028/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

### Semicolon body line indentation

Cases (1): `semicolon-body-line-indentation`.

Representative exact source, `semicolon-body-line-indentation` (`case.py`): JSON-escaped to preserve whitespace:

```json
"for x in items:\n    f();g()\n# fmt: off\nelse:\n    h()\n"
```

- Intended: RUF028 case.py:28–38. Pinned Ruff observed: none.
- Basis: The authored placement contract uses the physical indentation of the semicolon-separated loop-body line. Primary evidence: [cpython310-observation.json](../../profiles/RUF028/evidence/cpython310-observation.json).
- Reproduce: use the [profile](../../profiles/RUF028/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

### Unicode byte spans

Cases (1): `unicode-byte-spans`.

Representative exact source, `unicode-byte-spans` (`case.py`): JSON-escaped to preserve whitespace:

```json
"label=\"é🧪\"\nx=1 # fmt: on\n"
```

- Intended: RUF028 case.py:19–28. Pinned Ruff observed: none.
- Basis: The authored placement contract rejects an inline fmt: on after code. Primary evidence: [cpython310-observation.json](../../profiles/RUF028/evidence/cpython310-observation.json).
- Reproduce: use the [profile](../../profiles/RUF028/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.

### Comment at eof

Cases (1): `comment-at-eof`.

Representative exact source, `comment-at-eof` (`case.py`): JSON-escaped to preserve whitespace:

```json
"x=1\n# fmt: skip"
```

- Intended: RUF028 case.py:4–15. Pinned Ruff observed: none.
- Basis: The authored formatter-placement contract rejects an own-line fmt: skip at EOF because it trails no code. Primary evidence: [cpython310-observation.json](../../profiles/RUF028/evidence/cpython310-observation.json).
- Reproduce: use the [profile](../../profiles/RUF028/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.


## Explicit profile context replay (2026-11-05)

The canonical profile now states `project-context-mode = "isolated"` explicitly. Full-inventory staging had already selected this same isolated context; adding it changed the raw profile digest and caused strict certificate admission to refuse the prior implicit-default profile identity.

All 4 certified discrepancy cases were rerun against the approved Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`). Their complete reference diagnostic multisets and normalized invocation settings match their existing seals exactly. The [raw reference captures](../../results/discrepancy-profile-context-2026-11-05/RUF028/reference.json) retain argv, environment overrides, exit status, stdout, stderr, observed findings, settings and original certificate identities. The [migration record](../../results/discrepancy-profile-context-2026-11-05/profile-migration.json) binds the old and explicit-context profile hashes.

Only the certificate's `profile-sha256` changed. Source bytes, intended and reference expectations, executable pin, settings identity, and independent language-evidence artifacts remain unchanged; those artifact hashes were revalidated. No candidate execution or fresh independent-language execution is claimed by this reference-only replay, and no upstream bug status is inferred.
