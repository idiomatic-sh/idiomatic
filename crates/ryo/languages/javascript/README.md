# idiomatic-sh-ryo-javascript

JavaScript for Ryo: `.js`, `.jsx`, `.mjs` and `.cjs` source files parsed by Oxc 0.142.0 and represented as `ESTree` ECMAScript 2025 trees with the JSX extension. `.mjs` uses module grammar, `.cjs` uses script grammar, and `.js` and `.jsx` detect module syntax. `.jsx` and `.js` read JSX (Babel and the React toolchains read JSX in `.js`; `<` where an expression may start is JSX, and `a < b` stays a comparison); `.mjs` and `.cjs` do not. Oxc semantic syntax checking supplies early-error diagnostics as well as parser diagnostics.

## Usage

Link the language with `ryo_javascript::linked()` and hand the result to the SDK's calls.

```rust
let language = ryo_javascript::linked();
assert_eq!(&*language.name().0, "javascript");
```

## Domains

### extras

The javascript domain's `extras` concept: the kinds of its trees that its standard has no name for.

### `fact_notes`

Notes the modules each file imports when it is parsed, for the project to read back; the `notes` port writes them as bytes.

### language

The javascript domain's `language` concept.

### links

Links each import to the file of the repository it names, as Node resolves a relative specifier: the file, then itself with `.js`, `.mjs`, `.cjs` and `.jsx`, then its directory's `index` with each.

### manifests

Reads repository package dependencies, workspace membership, file ownership, the file each import names, and the files whose imports were examined.

### nodes

The javascript domain's `nodes` concept: the nodes of its trees, named as its standard names them.

### relations

The javascript domain's `relations` concept: one reader for each relation the language reads.

### syntax

The javascript domain's `syntax` concept.

### tokens

The javascript domain's `tokens` concept: the tokens of its trees, named as its standard names them.

## Differences from the standard

The AST follows `ESTree`. Lexical leaves include Acorn-compatible token categories, comments and white-space extras so the complete source survives the projection. Invalid inputs produce Oxc recovery trees and diagnostics; a refused Acorn input is compared as a refusal rather than as an arbitrary recovered tree. Parentheses are represented by tokens rather than extra AST nodes. Comments expose content without delimiters; string literals and identifiers expose decoded values. Private call names retain their `#` prefix.

Independent Acorn 8.15.0 evidence, with acorn-jsx 5.3.2 for JSX, checks the entire tree and the supported semantic facts. The suite contains 69 upstream inputs, a real Acorn CLI source, three JSX and comparison sources, 16 relation fixtures and 59 vocabulary examples. This is pinned acceptance coverage, not a claim to exhaust every ECMAScript program.

Readers implemented: `syntax_tree`, `syntax_child`, `syntax_error`, comment, `string_literal`, function, parameter, `static_call`, `static_import`, `assigns`, `defines`, `directive`, `inherits`, `reference`, `scope` and `visibility`. Calls requiring evaluation and anonymous bindings retain an explicit unknown value. ECMAScript has no type-annotation construct.

`source_dependency` links each `import` and `export ... from` declaration, `import()` of a string and `require()` of a string to the file of the repository its relative specifier names; a package, of the repository or outside it, is no row (`package_dependency` answers it), and a relative or `#` specifier that names no file reads the relation in part at its file. `analyzed_source` is every file the parse noted. No manifest is read for either.

Decorators, mentions and annotations are `NoSuchConstruct` (ECMAScript has none), mappings, entries and sequences `NotAKeyedDocument`, and prose `NotADocument`. TypeScript syntax is outside this provider; it has no dependency on another language provider.

Package dependencies, workspace members and the package of a file are read from the manifests npm, pnpm and Bun write, as the TypeScript provider reads them with its own copy of the readers: `package.json` in strict JSON, with its `workspaces` (npm, Yarn and Bun) and its `catalog` and `catalogs` (Bun), and `pnpm-workspace.yaml` with its `packages` and its catalogs. A dependency no package of the repository answers is a row with no target. When both providers are linked they write the same rows of one manifest, which the engine keeps once.
