# requests-carry-no-settings 0.1.0

The first release.

A field the entry lists in `settings` (`workers`, `memory`, `memory_budget`, `cache`, `build`,
`logging`) of a struct whose name the entry's `requests` globs match (`*Request`, `Session`) is a
finding. The field is found under its struct through the syntax tree.
