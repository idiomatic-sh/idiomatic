# composition

## 0.2.0

`compose` is `pub(crate) fn compose`, where it was `pub`: a command's or an entry's composition.rs sits in a private module and only its sibling command calls it, so `pub` was unreachable and rustc's `unreachable_pub` (on in `cargo-workspace` 0.11.0) reported it. The template names the visibility, so a placed composition is held to `pub(crate)` exactly. A product moves by placing `command` 0.2.0 (through `cli` 0.12.0) and writing `pub(crate) fn compose`.

## 0.1.0


- A composition renders without connections (`compose(parts)`) unless the verb has a `connections.rs` (`present["connections"]`).
