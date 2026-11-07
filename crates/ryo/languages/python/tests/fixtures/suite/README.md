# python test suite

## Source

The test resources of ruff's parser, `crates/ruff_python_parser/resources` of astral-sh/ruff (github.com/astral-sh/ruff, MIT License,
Copyright (c) 2022 Charles Marsh), copied unchanged: `valid/` and `inline/ok/` are in `valid/` (the latter as `valid/inline/`),
`invalid/` and `inline/err/` in `invalid/` (the latter as `invalid/inline/`). The `.editorconfig` of the resources is left out: it is
no Python. Every other file is here unless `NOTICE` names it as left out, with the reason.

## Version

The commit `4683270c69cbdaf6ad64d3f983fc458d5a43b41f`, the one the published crate `ruff_python_parser` 0.0.18 names in its
`.cargo_vcs_info.json`.
