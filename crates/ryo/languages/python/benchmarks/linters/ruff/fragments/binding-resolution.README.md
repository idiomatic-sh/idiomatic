# The event in effect for a read

`binding-resolution(reads=<reference, file>)` gives, for each given read or deletion and each namespace on its lookup path, the one event of the name that is in effect there: the latest one in schedule order before the read. This is a selection, and it is this pack's: Python has no single "event in effect" for a name whose bindings are alternatives (`if c: x = 1 / else: x = 2`), so the Python provider states the facts (`binding-event`, `binding-read`, `scope`) and the selection is made here.

The output has the columns of the provider relation it replaces (`reference`, `event`, `kind`, `scope`, `lexical`, `depth`, `ordinal`, `context`, `file`), so `ordinary-read-uses` can read it unchanged.

## Not selected here yet

The provider relation also answers two things of PEP 695 that this fragment does not: the type parameters of a definition as a namespace of their own (for a read in the definition's header, and between the definition and the scope around it), and a read in a type parameter's bound or default, which sees the end of the schedule. Until the fragment selects them, `ordinary-read-uses` keeps reading the provider relation, and the four rules that use it (F401, F811, F821, F841) are unchanged. On files without type parameters the fragment's rows are the relation's.

## What is read from the facts

- The lookup path of a read: the scope the read is evaluated in (`binding-read.scope`), then each enclosing scope up to the module (`scope`), a class being on the path only when the read is evaluated in it. `depth` is the place of a namespace on that path, from 0.
- A scope that declares the name `global` (a `global` event written in it) ends the path: the module follows it directly, one deeper.
- The read's point is its own event's `ordinal`; its `context` is `binding-read.context`.

## The selection

In each namespace on the path, the answer is the event of the name with the greatest `ordinal` below the read's point, among:

- the namespace's own `bind`, `annotation`, `cleanup`, `delete`, `global` and `nonlocal` events;
- the `global` and `nonlocal` events written in it that route the name elsewhere; the first of these also answers a read that comes before it.

A wildcard import answers nothing. Alternatives are not merged: one event answers. A deletion asks its own namespace for the event before itself.

`ordinal` is compared for order only; its values are a schedule index and mean nothing else.

## Cost

One `closure` over the scopes of a file gives the paths. The latest event is found by `latest-before`: events and probes of one (namespace, name) are paired in order and `resolve-pointers` follows each probe back to an event in 20 doubling rounds, so a (namespace, name) with N events and probes costs O(N).
