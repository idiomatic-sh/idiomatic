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
