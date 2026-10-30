# markdown standard

## Source

The standards that define the nodes and the tokens of the language's trees, copied unchanged: mdast, the specification of the Markdown syntax tree (`mdast.md`, from github.com/syntax-tree/mdast, MIT License, Copyright (c) Titus Wormer), which names the nodes; and the token types of micromark (`micromark-util-types.d.ts`, from github.com/micromark/micromark, MIT License, Copyright (c) Titus Wormer), which names the tokens.

## Pinned trees

`<path>.mdast` is the tree `mdast-util-from-markdown` gave for the file at the same path below `tests/fixtures` (`corpus/<file>.md`, `suite/valid/<section>/<example>.md`), printed by `unist-util-inspect`, recorded once with the tool and versions below and never edited: 672 files, every file of `corpus/` (12), every file of `suite/valid/` (652) and the input of every relation but `syntax_error` (8, as `<relation>.mdast` for `relations/<relation>/input.md`). The input of `syntax_error` is not UTF-8, so the tool, which reads text, has no tree of it. The language's tree of the same file, projected onto mdast (the tokens and markup dropped, a soft break joined into its `text`, the lines of a tight list item put in the `paragraph` mdast puts them in, the alternative text of an image and the text of front matter left as fields), must have the same node types in the same nesting (`adapter_test.rs` of the adapter). The positions, fields (`depth`, `url`, `checked`) and values of the print are pinned but not compared. Four files are the stated exceptions, listed by `adapter_test.rs` and in the README of the language:

- `corpus/ruff_CONTRIBUTING.md`: the line `[!NOTE]` of an alert is a paragraph of text in mdast, the marker of the quote in our tree.
- `suite/valid/link-reference-definitions/example-204.md` and `suite/valid/links/example-544.md`: a label defined twice is two `definition` nodes in mdast; `pulldown-cmark` lists the first only.
- `suite/valid/setext-headings/example-098.md`: `---` over `---` is empty front matter (`yaml`) in mdast; `pulldown-cmark` reads two thematic breaks.

The extensions the tool reads with are the ones the adapter's options turn on: tables, strikethrough, task list items and footnotes of GFM, math, and YAML and TOML front matter. The GFM autolink literal (bare URLs made links) is not among them, as `pulldown-cmark` has no such extension.

The tool, from a scratch directory outside the repository (`npm install mdast-util-from-markdown mdast-util-gfm-table mdast-util-gfm-strikethrough mdast-util-gfm-task-list-item mdast-util-gfm-footnote micromark-extension-gfm-table micromark-extension-gfm-strikethrough micromark-extension-gfm-task-list-item micromark-extension-gfm-footnote mdast-util-math micromark-extension-math mdast-util-frontmatter micromark-extension-frontmatter unist-util-inspect`), run as `node dump.mjs <file.md> <file.mdast>`:

```js
import {readFileSync, writeFileSync, mkdirSync} from 'node:fs'
import {dirname} from 'node:path'
import {fromMarkdown} from 'mdast-util-from-markdown'
import {gfmTable} from 'micromark-extension-gfm-table'
import {gfmTableFromMarkdown} from 'mdast-util-gfm-table'
import {gfmStrikethrough} from 'micromark-extension-gfm-strikethrough'
import {gfmStrikethroughFromMarkdown} from 'mdast-util-gfm-strikethrough'
import {gfmTaskListItem} from 'micromark-extension-gfm-task-list-item'
import {gfmTaskListItemFromMarkdown} from 'mdast-util-gfm-task-list-item'
import {gfmFootnote} from 'micromark-extension-gfm-footnote'
import {gfmFootnoteFromMarkdown} from 'mdast-util-gfm-footnote'
import {math} from 'micromark-extension-math'
import {mathFromMarkdown} from 'mdast-util-math'
import {frontmatter} from 'micromark-extension-frontmatter'
import {frontmatterFromMarkdown} from 'mdast-util-frontmatter'
import {inspect} from 'unist-util-inspect'

const [from, to] = process.argv.slice(2)
const tree = fromMarkdown(readFileSync(from, 'utf8'), {
  extensions: [gfmTable(), gfmStrikethrough(), gfmTaskListItem(), gfmFootnote(), math(), frontmatter(['yaml', 'toml'])],
  mdastExtensions: [gfmTableFromMarkdown(), gfmStrikethroughFromMarkdown(), gfmTaskListItemFromMarkdown(), gfmFootnoteFromMarkdown(), mathFromMarkdown(), frontmatterFromMarkdown(['yaml', 'toml'])]
})
mkdirSync(dirname(to), {recursive: true})
writeFileSync(to, inspect(tree, {color: false, showPositions: false}) + '\n')
```

## Version

mdast at the commit `c034ec9ddb8ca2ab16de90d668433624bdd92179` of 2026-02-04; micromark at the commit `6577c200155e9c6b85a42b26d298f8affc348f2f` of 2026-09-26. The extensions mdast and micromark have for front matter, math and task lists are not pinned: the kinds of those constructs are the language's extras.

The pinned trees: Node `v24.20.0`; `mdast-util-from-markdown` 2.1.0 (with `micromark` 4.0.3); `micromark-extension-gfm-table` 2.1.2, `-gfm-strikethrough` 2.1.0, `-gfm-task-list-item` 2.1.0, `-gfm-footnote` 2.1.0, `-math` 3.1.0, `-frontmatter` 2.0.0; `mdast-util-gfm-table` 2.0.0, `-gfm-strikethrough` 2.0.1, `-gfm-task-list-item` 2.0.0, `-gfm-footnote` 2.1.0, `mdast-util-math` 3.0.0, `mdast-util-frontmatter` 2.0.1; `unist-util-inspect` 8.1.0.
