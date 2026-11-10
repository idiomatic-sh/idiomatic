# PLR2044 source candidate

The strict singleton profile passes all 50 full-report cases against pinned Ruff
0.16.5 on sealed worker6540. It uses Python3.10, canonical `always` applicability,
and the exact existing two-module suppression closure. These are source-proof
results pending integration registration, with no broader parity or performance claim.

An empty comment has no content after its first `#` except Python lexical
whitespace: space, tab and form feed. Other Unicode whitespace is content;
additional hashes are content. The diagnostic retains the real comment token's
entire byte span, including its trailing whitespace and excluding CR/LF endings.
Strings and f-strings do not supply comment rows.

The block-comment exemption is independent rule policy. Consecutive own-line
comments in the same raw UTF-8 byte column form a component. An empty member is
exempt only if that component contains a nonempty member. Blank lines, code,
trailing comments and column changes break components. Visual tab expansion is
not substituted for the raw source column. A block of only empty comments still
reports every member.

The query uses parser-owned comment IDs, physical token text and source locations.
A comment is own-line when no preceding token has non-whitespace text ending on
that same physical line; tokens ending at column1 after a newline do not block it.
This tests text directly without applying a regular expression to a Span. Closure
is limited to adjacent same-column comment IDs; diagnostic spans remain the real
comment-token spans.

The 50 full-report fixtures contain 48 independently authored controls and two
exact upstream source fixtures. Their 42 intended diagnostic spans include nine
ranges transcribed from the source-pinned snapshots. Controls cover CRLF and CR,
Unicode byte offsets, BOM prefixes, inline comments, explicit continuation,
blank/code/column boundaries, empty-only blocks, tabs/form feed, strings, and
ordinary suppression. TOML, declared module closure, byte ranges and source
identity are sealed by canonical stage_binding and native verify-cases receipts:
50complete pairs,50raw exact matches,0unknown/incomplete/skipped. Runtime artifacts:
plr2044-typed-correction-20261102/profiles/PLR2044; worker SHA
6540ecd69e387437f7e2a7a03eebaeb24544ec97bd0dcfba9520ca7e46e4f83c.

Primary source audit uses Ruff tag `0.16.5`, commit
`9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`:

- [Rule, diagnostic span and empty-comment policy](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/pylint/rules/empty_comment.rs).
- [Own-line and block-comment policy](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_python_trivia/src/comment_ranges.rs).
- [Python lexical whitespace](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_python_trivia/src/whitespace.rs).
- [Main fixture](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/resources/test/fixtures/pylint/empty_comment.py), SHA256 `7381bbb239693413de86329a97927d248c6bdeea796ec8e462d4ebb06d832dca`.
- [Main snapshot](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/pylint/snapshots/ruff_linter__rules__pylint__tests__empty-comment_empty_comment.py.snap), SHA256 `e19fa434eee460f74138b6951356af4c0e5d05f2055e91664b7719244c607fc0`.
- [Continuation fixture](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/resources/test/fixtures/pylint/empty_comment_line_continuation.py), SHA256 `eb906ad04f04d9b6dffa8d65031466040d94c0c630e50a03c581a8037eefb63f`.
- [Continuation snapshot](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/pylint/snapshots/ruff_linter__rules__pylint__tests__empty-comment_empty_comment_line_continuation.py.snap), SHA256 `d6f0fc1f3c68536497e79e591cd62fddfc4885595d2d638e7cc474916b701a93`.

## Deferred RUF028 audit

RUF028 was inspected as a possible second rule. Its complete formatter-specific
policy uses a source-order comment attachment walk, not simply the smallest AST
range containing a comment. Comments can attach on node exit beyond that node's
end when only trivia intervenes, with own-line indentation affecting attachment.
The policy additionally distinguishes decorator gaps and the first statement of
an alternate body. A containing-node approximation would misclassify these
cases. No partial RUF028 query is shipped, and no capability blocker is claimed.
This remains source-authoring work with existing facts and requires a separate
complete attachment model and fixtures.

Follow-up sources: [attachment walk](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/ruff/rules/suppression_comment_visitor.rs),
[full rule policy](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/ruff/rules/invalid_formatter_suppression_comment.rs).
