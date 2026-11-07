# composition-opens-nothing 0.1.0

The first release. The command spec says a composition only constructs and opening a store
is the connections' I/O; this rule enforces it. Tastemaker's `check` and `validate`
compositions opened the source inventory while building the service.

## What it reads

- In a `composition.rs`: a call of a path whose last segment is `open` or `connect`, or
 starts `open_` or `connect_`, and a method call of such a name.

## Not seen

- Opening under another name; direct filesystem access is `io-at-boundary`'s.
