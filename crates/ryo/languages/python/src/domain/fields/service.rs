//! The `fields` service: free functions named by verb that say the role of an element under its parent, by the field of
//! `Python.asdl` that holds it. The library's source-order walk writes each child without its field, so a role is read from the kinds
//! of the parent and the element, from where the element stands among the nodes under the parent, and from the keyword or the
//! punctuation before it (`else`, `finally`, `from`, `as`, `:`, `/`, `*`), as `CPython`'s parser tells the fields apart. The adapter's
//! cases hold every role to what `CPython`'s `ast.dump` prints for the pinned files.













