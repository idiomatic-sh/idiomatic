# no-packed-strings 0.1.0

The first release. A field that holds several values joined into one string has to be split
again by whoever reads it, and loses the values' types: Asymptote's `at = "path:line:column"`
and its lock stamp `length:modified`, Tastemaker's `identity#check` rule ids, ryo's
`name<tab>summary` listings and `file#position` node cells.

## What it reads

- A string literal of `format!`, `write!`, `writeln!` or `format_args!` that is two or more
 placeholders with only `:`, `,`, `;`, `#`, `=`, `|` or a tab between them, and no space.
- It is reported when the nearest enclosing struct literal field, tuple struct call (a call
 whose last segment starts with a capital, `Some`, `Ok` and `Err` aside) or `let` is a field
 or a call: text composed in a local first is read as text for a person.

## Not seen

- Packing by `push_str`, `join` or `concat!`, or through a local.
- `/` as a separator, which is also how a path is written.
