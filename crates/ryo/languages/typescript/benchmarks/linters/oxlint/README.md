# oxlint benchmark

The TypeScript provider against oxlint 1.76.0, the standalone executable of the GitHub release `apps_v1.76.0`, which is built on
`oxc_parser` 0.142.0, the parser both ryo providers link. `uv run --script tools/install.py` installs it as `oxlint`.

`benchmark.toml` pins the 96 rules a plain run of oxlint 1.76.0 enables. Of them, 18 have a query, each in `rules/<plugin>.<rule>/`
with its cases; the others are `deferred`, blocked by a relation the TypeScript provider does not answer yet, or `pending`.
`fragments/eslint-directives.ryoql` holds the `eslint-` and `oxlint-` disable directives every query honors, as oxlint reads them.

The reference runs `oxlint -A all -D <plugin>/<rule>… --format json --threads 1 --disable-nested-config -- <files>` in the corpus
root over TypeScript files only (`.ts`, `.mts`, `.cts`, `.tsx`); the worker links `ryo_typescript` alone.

From `crates/ryo`, with the pinned executable on the path:

```sh
cargo bench -p idiomatic-sh-ryo-typescript-oxlint-benchmark --bench oxlint -- verify-reference-cases "$(command -v oxlint)"
cargo bench -p idiomatic-sh-ryo-typescript-oxlint-benchmark --bench oxlint -- verify-cases "$(command -v oxlint)"
cargo bench -p idiomatic-sh-ryo-typescript-oxlint-benchmark --bench oxlint -- audit
```

`verify-reference-cases` checks the cases against oxlint alone; `verify-cases` runs both contestants and refuses while any default
is `pending`. The `measure` and `diagnostic-measure` commands take the release executable, a fresh cache directory and the resource
collector, and clone hono and vite at the revisions `../../benchmark.toml` pins.
