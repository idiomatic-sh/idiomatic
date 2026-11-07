# no-whole-state-ports 0.1.0

The first release. A store port reads and writes the core's own rows, never one value
holding everything the store keeps: the `Image` that Asymptote's cache port, ryo's image
port and Tastemaker's store port load and save is rewritten whole when one row changes.

## What it reads

- An identifier `Image`, or ending `Image`, inside the parameters or the result of a
 function in a port's `interface.rs` under `outbound/`.

## Not seen

- A whole-state type under another name.

## Amended in place: flat tables together

Owner 2026-10-29 ("fix the spec yes"), with the `port` spec 0.2.0: a store port may hand over its product's flat
tables together (`load() -> Tables`, `replace(Tables)`) when every run replaces them all through a temporary file
renamed over the store. The why and help say so; the query is unchanged. Scenarios `flat-tables-replaced-whole`
(passes) and `nested-image-replaced` (a nested `StoreImage` loaded and replaced: fails). A write that replaces some
tables and keeps the rest is added to what the rule does not see.
