# css test suite

## Source

css-parsing-tests, the implementation-independent tests of CSS Syntax Level 3 by Simon Sapin (https://github.com/SimonSapin/css-parsing-tests),
as rust-cssparser vendors and runs them in `src/css-parsing-tests` (https://github.com/servo/rust-cssparser), dedicated to the public domain
(`LICENSE`). Its files are JSON lists of an input and the expected result; the inputs of the two that parse a whole stylesheet are the
files here, one each, named by their place in the list (`NOTICE`).

## Version

rust-cssparser tag `v0.38.0`, commit `d128a90e1203fc4afb959420d6dc5e24f77a3191` (2026-09-07), the version of the `cssparser` crate the language
pins.
