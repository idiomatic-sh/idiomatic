# TypeScript reference

The independent reference is typescript-eslint's [AST specification](https://typescript-eslint.io/packages/typescript-estree/ast-spec/), read by `@typescript-eslint/typescript-estree` 8.72.0 with TypeScript 6.0.3. `package-lock.json` pins the complete dependency graph. The runtime adapter uses Oxc; this reference uses the TypeScript compiler.

| File | Source |
| --- | --- |
| `ast-node-types.ts`, `ast-token-types.ts` | Unchanged inventories from typescript-eslint v8.72.0, commit `a4c2603d3e4fd43b33bc25b82457716e9f7cef71`, `packages/ast-spec/src/` |
| `NOTICE-LICENSE` | The upstream MIT license at that commit |
| `../suite/` | Unchanged upstream ast-spec fixtures; `suite/NOTICE` maps every destination to its upstream path |
| `dump.mjs` | Reference reader, producing AST nodes, tokens, comments and semantic facts independently of the runtime adapter |
| `output.txt` | Pinned output for the fixture files |

From this directory, install with `npm ci --ignore-scripts`. From `tests/fixtures`, regenerate with:

```sh
find relations corpus suite -type f \( -name '*.ts' -o -name '*.tsx' \) | sort | xargs node standard/dump.mjs > standard/output.txt
```

Each `file HASH LENGTH` header uses the FNV-1a 64-bit hash and UTF-8 byte length. `refused` means the reference parser rejected the input; there is no reference recovery tree. Tree lines are `tree DEPTH N|T KIND START END`. Nodes retain AST parentage, including equal-range wrappers and overlapping shorthand children. Children sort by start ascending, end descending, preserving AST property order on ties. Tokens attach to the first deepest containing AST child. The root covers the entire source, and source gaps become `whitespace`. Positions and ranges use UTF-8 bytes; the reference parser's UTF-16 offsets are converted.

Facts use `comment FROM TO TEXT`, `string FROM TO VALUE`, `import FROM TO BINDING PATH`, `function FROM TO NAME`, `parameter FROM TO KIND ORDINAL NAME`, and `call FROM TO TARGET`. Points are one-based `line:byte-column`; text is `x` followed by its UTF-8 hex; `?` denotes an absent name. Function spans include the function expression/declaration, import spans include the import declaration, and call spans include only the callee. Parameters are zero-based, `variadic-positional` for rest parameters, `receiver` for a `this` parameter (TypeScript Handbook, "this parameters") and `positional` otherwise. String facts include all string Literal nodes, including module specifiers.

## Differences from the standard

AST kind names are kebab case. Tokens that collide with node names (`Identifier`, `PrivateIdentifier`, `JSXIdentifier`, `JSXText`) have `-token` appended. Whitespace and hashbang are extra kinds because AST_TOKEN_TYPES omits them. Hashbang ranges come independently from the pinned TypeScript compiler’s `getShebang`. The program's span extends over leading and trailing source gaps. Refused source has no whole-tree comparison because typescript-estree throws instead of returning a recovery tree.
