# constants-once 0.1.0

A string constant is declared in one file. A constant of the same name holding the same string,
declared in two or more files, is reported at each declaration.

A function of a `constants.rs` that returns a string is a constant written as a function: its string is
matched by value against every other declaration, under any name. A one-word value is a format's
vocabulary in an adapter: it is one concept only between two adapters, or between two files outside
adapters.

Constants are compared within one package: another package can name a constant only through a dependency,
and the crates of a language may not depend on one another.
