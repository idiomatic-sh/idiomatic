The policy follows Ruff 0.16.5 `custom_type_var_for_self.rs` and the shared
`analyze/class.rs` metaclass check. PYI019 requires definite metaclass status;
PYI034 skips definite and possible metaclasses. The shared helper preserves
ordered depth first edge visits, including calls visited before the first
successful base and edges into already visited classes. The method node is the
entry vertex so the starting class is initially absent from Ruff's seen set.

Qualified origins intentionally distinguish cached `.py` class bases, current
`.pyi` PYI034 analysis, and deferred `.pyi` PYI019 analysis. A deferred load before
a later deletion restores only the immediately shadowed non-unbound binding,
as Ruff `SemanticModel::resolve_load` does; a deletion before the load remains
unbound. `lookup-symbol-events` retains real event occurrences and lexical scope
identity. It handles transient annotation/header TypeScopes separately from real
namespace bindings. These are caller policy choices, not ordinary latest-event
lookup.

The retained suites contain 139 PYI019 cases and 72 PYI034 cases, including the
original 21 PYI034 cases, upstream fixtures, Unicode and parenthesized ranges,
class header/function annotation scopes, cyclic ordered traversal, aliases,
local members, rebinding, and deletion controls. On worker SHA
`65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b`, all 422
reference/candidate records are complete and pass (target Python 3.10). Receipts:
`/tmp/pyi019-sol-seven/cases.jsonl` and `/tmp/pyi034-sol-seven/cases.jsonl`.
This evidence covers the retained cases rather than universal semantic parity.
