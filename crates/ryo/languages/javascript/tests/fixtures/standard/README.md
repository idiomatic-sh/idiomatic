# JavaScript standard

## Source

ESTree ECMAScript 2025 syntax with the JSX extension, independently read by [Acorn](https://github.com/acornjs/acorn/tree/8.15.0) and [acorn-jsx](https://github.com/acornjs/acorn-jsx/tree/5.3.2). `dump.mjs` uses Acorn alone: it traverses every AST child, scans tokens and comments, converts UTF-16 offsets to byte offsets, and records the complete tree and semantic facts. Runtime code uses Oxc, never this dump generator or Acorn. `samples.json` retains the source bytes used to exhibit the closed kind vocabulary.

Lexical tokens attach to the first deepest containing AST node. Equal-range AST siblings remain distinct. White space is an extra because ESTree does not represent it. Template lexical segments combine Acorn's delimiter/content tokens into ECMAScript template head, middle and tail lexemes. Acorn syntax errors are explicit refusals, not empty trees.

The `tests-*.js` files are unchanged upstream suite definitions. Their `test` and `testFail` source arguments are pinned unchanged in `../suite/`; parser options are deliberately replaced by ECMAScript 2025 module parsing followed by script parsing. Reference output therefore records acceptance under this language contract, rather than historical upstream options.

`ast_kind.rs` is the unchanged Oxc 0.142.0 generated upstream AST enumeration, used to check the mapping census against the installed parser. It is independent of the Acorn tree reference. Oxc commit `fc702c1fa9f0412d06ec6908b58cd395b826cf7f` is MIT licensed; see `NOTICE-OXC-LICENSE`.

## Version

acorn-jsx 5.3.2 (MIT) extends it with JSX; a JSX tag name, attribute name and text are the `jsx-identifier-token` and `jsx-text-token` categories. Acorn 8.15.0 (MIT), tag object `b63a4e15ce4ca3d4c85025f7e25cd83337198f78`; Oxc 0.142.0 supplies the independent runtime parser.

Regenerate from `tests/fixtures`, using Node.js and Acorn installed outside the repository:

```sh
npm install --prefix /tmp/ryo-javascript-reference acorn@8.15.0 acorn-jsx@5.3.2
ACORN_MODULE=/tmp/ryo-javascript-reference/node_modules/acorn/dist/acorn.mjs \
ACORN_JSX_MODULE=/tmp/ryo-javascript-reference/node_modules/acorn-jsx/index.js \
  node standard/dump.mjs $(find relations suite corpus \( -name '*.js' -o -name '*.jsx' \) | sort) > standard/output.txt
```
