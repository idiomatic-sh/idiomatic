# idiomatic-sh-ryo-html

The HTML language of Ryo reads `.html` and `.htm` documents with SWC's HTML5 tokenizer and tree builder. It exposes document structure, comments, decoded attribute values, prose, sections, anchors, and URL-valued attribute references. JavaScript and CSS inside HTML remain HTML text; their embedded language relations are not inferred.

## Usage

```rust
let language = ryo_html::linked();
assert_eq!(&*language.name().0, "html");
```

## Domains

### extras

Source bytes discarded by tokenizer recovery.

### language

The HTML provider identity.

### nodes

Native DOM nodes and the source wrapper.

### relations

Readers of Ryo's existing relation catalog.

### syntax

Node kinds, scalar properties, and DOM navigation.

### tokens

Actual HTML tokenizer token categories.

## Differences from the standard

SWC constructs the DOM, including implied elements, foster parenting, namespaces, and template document fragments. Ryo adds one explicit `source-file` wrapper. Its first child is the complete native `document` subtree; subsequent children are actual tokenizer leaves in source order. This separates DOM order from lexical order while preserving both. Adjacent character tokens are grouped into a contiguous run, including multiple decoded characters originating from one character reference. Bytes discarded by tokenizer recovery are retained as `error` leaves. Concatenating all token source slices reproduces the original file exactly.

Attributes are native SWC attribute nodes under their element, preceding its DOM children. Duplicate attributes with the same namespace and local name retain the first occurrence, as required by the DOM; SWC also retains later source duplicates in its AST. Each scalar property is also a node of its own under its node, with no source range (`unlocated`), its kind the property (`name`, `namespace`, `prefix`, `value`, `public-id`, `system-id`, `document-mode`), its `syntax_tree.text` the property's value, and its `syntax_child.field` the property; an attribute is in the role `attribute` under its element. A property the DOM leaves empty has no node, except a value. So a rule reads an element's name or an attribute's value as structure: the `name` child of an `element`, the `value` child of an `attribute`. The published `syntax_tree.text` for a DOM node is a deterministic JSON array of property pairs: `1` local name, `2` namespace URI, `3` prefix, `4` decoded value, `5` public identifier, `6` system identifier, `7` document mode. This is output serialization of native fields; production readers access typed properties directly and never deserialize an AST. Token text remains the original source bytes. Nodes without scalar properties have empty text.

Native ranges remain unchanged. Token ranges describe the source consumed since the previous SWC tokenizer emission. They can include discarded markup or reconsumed lookahead: `</><p>`, `<<p>`, and CDATA closing delimiters exercise these differences from exact lexical token boundaries. These are the library’s actual ranges, not inferred locations. When SWC supplies no location for an implied node, Ryo stores explicit `NO_SPAN` with `ElementOrigin::Implied`; it never invents a zero-width source location. Queries render this as `unlocated`, location reporting leaves it unplaced, and numeric location comparisons do not treat it as an offset. Equality can compare two absent spans, consistently in lowered and ordinary queries. Native DOM preorder may differ from increasing source offsets; span joins account for that ordering.

The independent reference uses pinned parse5 8.0.1, supplemented by Chromium 140.0.7339.16 for three select-containing inputs: parse5 predates the current customizable-select tree construction rules. All 134 reference entries, including malformed inputs, are checked against actual emitted relation rows. The comparison retains the entire DOM, properties, attributes, namespaces, template contents and edges, and verifies all reconstructed source bytes. It deliberately does not equate the two parsers’ tokenizer boundaries. A separate independent traversal of SWC verifies every native kind, edge and range, including absent ranges and lexical leaves. The suite includes pinned html5lib tree-construction cases. `tests/fixtures/standard/dump.mjs` reproduces the independent dump; the standard README records the exact tools and limitations.

Parsing uses document mode with scripting enabled. Fragment-context parsing and execution of embedded scripts are outside this provider's document contract. A URL-valued attribute's `reference.target` is resolved to a path of the run as the language contract resolves a link (`href="../guide.html#setup"` in `docs/a.html` points at `guide.html#setup`; a fragment alone is the file's own path with it; a target with a scheme or a host stays as written; the first `<base href>` of the document is the base the relative URLs of the others are resolved against, a base outside the run leaves them as written, and a fragment alone stays the file's own, which a browser would resolve against the base), and the attribute's value keeps what the file writes; the provider does not fetch URLs. HTML has no dependency manifest.
