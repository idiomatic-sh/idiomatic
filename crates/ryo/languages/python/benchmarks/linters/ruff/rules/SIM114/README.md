The batch follows tagged Ruff 0.16.5 source and includes each upstream fixture:
B005 `strip_with_multi_characters.rs`, SIM102 `collapsible_if.rs`, SIM103
`needless_bool.rs`, SIM114 `if_with_same_arms.rs`, SIM117 `ast_with.rs`.

`statement-equal` compares original rooted semantic trees, stopping at expression
roots and using `expression-shape` for Ruff's structural expression equality.
It aligns every descendant by its complete field/ordinal path and requires
coverage in both directions. It retains statement identity, ordering, nested
structure, identifiers, imports, async forms, and pattern captures. SIM114 then
compares ordered comments after each test's last physical line through the last
body statement's last physical line. Comments on the test line are excluded,
matching upstream `body_range`; trailing body comments participate.

`conditional-guards` preserves Ruff's name/attribute heuristic for TYPE_CHECKING,
aliased typing imports, and any expression referring to sys.version_info or
sys.platform. The caller supplies the original if test, even when the finding
starts at its final elif branch.

Pinned policy quirks are intentional: SIM102's parent suppression compares the
child with the parent node due to local shadowing, so deep chains emit overlapping
findings. Its last-elif selection still checks the original if-body length.
SIM117 suppresses a nested with when its parent with has one body statement,
even if mixed async/sync or an exempt context prevents the parent diagnostic.
These behaviors differ from later working-tree source; retained controls cover
both. SIM103 handles explicit else, final elif+else, and immediate next-suite
return separately.

`repeated-characters` uses existing capture partitions of decoded strings. Each
character occurrence is identified by its distinct prefix, and single-character
captures use Unicode scalar matching including newlines. Captures require
nonempty slots, so first/last/interior positions are combined explicitly. This
also covers concatenation, decoded escapes, non-BMP text, and surrogate escapes.
The helper is restricted to actual candidate arguments. Capture enumeration is
quadratic in string length; this batch adds no engine operation.

On actual worker SHA
`65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b`, B00523,
SIM10217, SIM10317, SIM11458, SIM11714 total 129 cases / 258 complete records
pass, with no unknowns, errors, or skipped files. Receipts are
`<lowercase-rule>-sol-seven/cases.jsonl`. This is retained-case evidence,
not a universal parity claim. PLE1310 and PLW0406 have separate capability
receipts and are not part of this authored batch.
