# catch-all-names 0.1.0

The first version: a catch-all-names rule refusing common, utils, shared and helpers.

A finding is a file named `common`, `utils`, `shared` or `helpers`, with any extension or
none, and every file beneath a directory with one of those names, at any depth. The names
are matched whole, so `commons/`, `shared_state.rs` and `helper.rs` pass. It reads only
`filesystem.file`, so it holds for every kind of file.

It is placed by no spec, starter or product yet: 24 files still live under
`tests/common` and `benches/performance/common`, which the `crate` spec places. The lift
that files them into the primitives they are removes those
two imports and places this rule with `include = ["**"]`.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.
