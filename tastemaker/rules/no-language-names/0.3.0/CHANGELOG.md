# no-language-names 0.3.0

The languages denied are given as well as found.

The entry names them in `languages`: a product that holds no `languages/` directory, as the Tastemaker
and Asymptote cores, denied no language name at all in 0.2.0, so a comment naming one passed. The
directory names under `languages/` are still denied beside the ones given.

A language name is also denied as it is written, case-aware, in `written`: `Go` capitalized (the
lower-case `go` is the English verb, so it cannot be one of `languages`), a whole string `go` given
as a default, and a name the words split (`JavaScript`). The build manifests `go.mod`, `go.sum`,
`package.json` and `pyproject.toml` are read beside `Cargo.toml`.
