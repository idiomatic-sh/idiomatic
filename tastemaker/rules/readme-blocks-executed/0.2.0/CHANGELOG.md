# readme-blocks-executed 0.2.0

A README's `console` blocks run in a test: a `console` block is reported when no `tests/readme.rs` in scope hands
`../README.md` to trycmd. A shell block (`sh`, `bash`, `shell`, `zsh`) is a reference list of commands and is no longer
reported.

# readme-blocks-executed 0.1.0

A README's commands run in a test. A shell block (`sh`, `bash`, `shell`, `zsh`) is reported, since
trycmd never runs it, and a `console` block is reported when no `tests/readme.rs` in scope hands
`../README.md` to trycmd.
