# HTML standard reference

## Source

The [WHATWG HTML Living Standard](https://html.spec.whatwg.org/multipage/parsing.html) defines tokenization and tree construction. SWC constructs the production DOM. Independent parse5 and Chromium implementations supply expected DOM trees.

## Version

`package-lock.json` pins parse5 8.0.1 and its dependencies. Run `npm ci` here, then `node dump.mjs ../../.. /path/to/chrome` to regenerate `parse5.json` and `chromium.json`. The browser must report exactly `Chromium 140.0.7339.16`. The generator records tokenizer deliveries from the actual parse5 parser, preserving parser-controlled tokenizer states.

For the three select-containing sources recorded in `chromium.json`, Chromium supplies the DOM tree because parse5 8.0.1 predates the current customizable-select parsing rules. Chromium traverses the real `DOMParser` result; the generator implements no tree builder. This inert-document path rejects select fixtures containing `noscript`, whose interpretation would depend on the scripting flag. All other sources use parse5 with scripting enabled. Document mode is obtained from parse5 for every input.

`html5lib-tests1.dat` is copied unchanged from html5lib/html5lib-tests commit f994590f528ac8b6073665791ddb1ed85c66dfb2, `tree-construction/tests1.dat`. Its 112 document-mode examples are retained, with their license. Fragment-context parsing is outside this provider's contract.

The 134-entry dump retains the complete DOM, attribute order, namespaces, decoded properties and template fragments. The comparator reconstructs these from actual emitted `syntax_tree` and `syntax_child` rows; it does not reparse with SWC. It compares the complete DOM subtree and concatenated original lexical source. Token boundaries differ between implementations, so the independent native SWC walk separately checks every kind, edge and source range. An adapter case explicitly checks every dump entry, including invalid fixtures that the shared conformance rail otherwise checks only for diagnostics.
