# PLE0643 source candidate

For a literal list or tuple with known written cardinality, this query reports
the real index-expression span when a direct integer index is at least its
length, or a direct unary-minus integer has magnitude greater than its length.
The boundary `-length` is valid for a nonempty sequence. `-0` remains zero, so an
empty sequence indexed by `-0` reports. Decoded arbitrary integers are compared
exactly; no machine-width overflow is used as a substitute for arithmetic.
Booleans, floats, unary plus, compound expressions, slices and nonliteral
receivers are outside this rule's index/receiver classification.

A direct starred element makes the receiver's cardinality unresolved for this
query. The rule declines that receiver rather than count a starred expression
as one element. A starred expression nested inside an ordinary element does not
change the outer literal's cardinality. This is an explicit independent safety
policy, not a claim that arbitrary iterable expansion has a known length.
No recursive constant evaluator or decoded-string character count is invented.

The source-pinned reference counts syntactic elements without a starred guard.
For example, with `items = [10, 20]`, `[*items][1]` accesses a valid element.
Our intended output is empty. A possible reference discrepancy is only a source
prediction until the canonical reference run and independent proof are captured;
no reference exception or admission is supplied here.

All cardinality work is keyed by the sequence owner and file. There are no
all-pair token scans. The singleton Python 3.10 profile uses the pinned
accepted-syntax phase and the existing exact suppression closure. Source cases
include the tagged upstream fixture intact and independent signed, empty,
arbitrary-precision, starred, Unicode/CRLF and suppression controls.

Policy source:
[`potential_index_error.rs`](https://github.com/astral-sh/ruff/blob/0.16.5/crates/ruff_linter/src/rules/pylint/rules/potential_index_error.rs),
Ruff 0.16.5 commit `9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`.
The candidate remains unadmitted pending canonical candidate/reference evidence.
