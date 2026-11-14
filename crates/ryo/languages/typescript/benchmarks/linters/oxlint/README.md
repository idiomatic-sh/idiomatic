# oxlint benchmark

The TypeScript provider against oxlint 1.76.0, the standalone executable of the GitHub release `apps_v1.76.0`, which is built on
`oxc_parser` 0.142.0, the parser both ryo providers link. `uv run --script tools/install.py` installs it as `oxlint`.

`benchmark.toml` pins the 96 rules a plain run of oxlint 1.76.0 enables. Of them, 90 have a query, each in `rules/<plugin>.<rule>/`
with its cases; the other 6 are `deferred`, each under the limitation that blocks it: a relation or column the TypeScript provider
does not answer yet, or, for `typescript_silent`, a rule oxlint never reports on a TypeScript file.
`fragments/eslint-directives.ryoql` holds the `eslint-` and `oxlint-` disable directives every query honors, as oxlint reads them;
`fragments/parenthesized.ryoql` gives the parentheses written around an expression, which the tree keeps as punctuators only;
`fragments/constness.ryoql` gives the expressions whose truthiness is constant, as oxlint's `is_constant` decides it;
`fragments/unique-bindings.ryoql` gives the one binding a reference resolves to, with its kind;
`fragments/constant-values.ryoql` gives the value a `const`, `let` or `var` initializer gives a reference;
`fragments/regexp-without-global.ryoql` gives the regular expressions written without the `g` flag.

These oxlint checks have no query and no case yet:

- `eslint.no-unused-vars`: discarded reads; writes to a loop variable, exempt only in a for-in or for-of loop whose body returns
  at once; `import x = require()`; the type parameters of an ambient interface outside a `declare` module block; `declare global`,
  found by the module name `global` alone.
- `oxc.only-used-in-recursion`: a recursive use written as JSX.

The reference runs `oxlint -c <state>/oxlint.json -A all -D <plugin>/<rule>… --format json --threads 1 --disable-nested-config --
<files>` in the corpus root over TypeScript files only (`.ts`, `.mts`, `.cts`, `.tsx`); the worker links `ryo_typescript` alone. The
adapter writes `{}` to `oxlint.json` in the run's state before each run, and discovery (`oxlint -c <scratch>/oxlint.json
--debug=files --disable-nested-config .`) reads the same, so a project's own configuration is never read: vite's `oxlint.config.ts`
needs a JavaScript runtime this standalone executable does not have.

From `crates/ryo`, with the pinned executable on the path:

```sh
cargo bench -p idiomatic-sh-ryo-typescript-oxlint-benchmark --bench oxlint -- verify-reference-cases "$(command -v oxlint)"
cargo bench -p idiomatic-sh-ryo-typescript-oxlint-benchmark --bench oxlint -- verify-cases "$(command -v oxlint)"
cargo bench -p idiomatic-sh-ryo-typescript-oxlint-benchmark --bench oxlint -- audit
```

`verify-reference-cases` checks the cases against oxlint alone; `verify-cases` runs both contestants over the authored rules and
refuses while any default is `pending`. The `measure` and `diagnostic-measure` commands take the release executable, a fresh cache directory and the resource
collector, and clone hono and vite at the revisions `../../benchmark.toml` pins.
