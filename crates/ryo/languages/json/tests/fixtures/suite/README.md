# json test suite

## Source

JSONTestSuite (github.com/nst/JSONTestSuite, Nicolas Seriot, "Parsing JSON is a Minefield"), its `test_parsing` directory: the `y_*` files under `valid/` and the `n_*` files under `invalid/`. The `i_*` files, which the suite leaves to the parser, are left out (`NOTICE`).

Read as `.json`, the files are json as RFC 8259 defines it: each `valid/` file gives no syntax error and each `invalid/` file at least one, among them the files the library reads as json with comments (a trailing comma, a comment) that the language reports in this dialect.

## Version

Commit 1ef36fa01286573e846ac449e8683f8833c5b26a of the repository.
