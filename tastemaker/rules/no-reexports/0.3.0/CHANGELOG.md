# no-reexports 0.3.0

The rule limits reexports to the actual SDK spec surface.
The working rule no longer accepts `derive-facade`; ordinary crate roots and CLI
code receive the same prohibition. Existing facade fixtures now fail intentionally,
including derives with the former exact cfg/expect shape. Products and the working
devtool starter exclude only the src tree their SDK placement owns.

## Earlier working shape

Checks exactly what 0.2.0 checks; only where the match of imported names to derive names
is switched changes.

## Changed

- The match of a facade's names to `#[proc_macro_derive]` names, which reads names across
 files, now starts from the names of entries with `derive-facade = true`
 (`facade-candidate-names`) instead of switching the facades off after the match. The
 entry on every other file (`derive-facade = false`) then never relates two files, so
 Ryo answers it one file at a time: on crates/ryo that entry held every file's syntax
 at once (15 GB, a 5 GB cache and most of `tm workspace check`'s time).
- An entry with `derive-facade = false` no longer waits on every reached file's coverage
 to report a forwarding declaration: a file Ryo could not read fully leaves only its own
 findings unknown.

## Moving up from 0.2.0

Change both entries' `version` to `"0.3.0"`; their params and paths stay.

## Fixtures

The 0.2.0 fixtures, unchanged.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.

The type-alias arm fires only when the aliased type is a path (`pub type Counter = usize;`): an alias of a `fn` pointer or a `dyn Fn` forwards no other module's item, so it is not a reexport.
