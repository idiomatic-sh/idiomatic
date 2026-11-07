# Ryo

The `ryo` command line and its SDK: query a repository's source with RyoQL, over independent language providers.

Engine extension ownership and current enforcement are recorded in the [contracts map](docs/contracts/README.md).

## Install

| Language | Command |
| --- | --- |
| rust | `cargo install --locked idiomatic-sh-ryo` |
| rust, the SDK as a library | `cargo add idiomatic-sh-ryo-sdk` |

The command is `ryo`; the SDK's library is `ryo`.

## Usage

`ryo query run` runs a query over the repository at `--root` and prints its own columns, then the line and the character of each row. On a terminal the text goes through the pager, `PAGER` or `less -FRX`; a pipe or a file gets it whole:

```console
$ ryo query run --root languages/rust/tests/fixtures/manifests/packages --text 'let f = select name, file, span from "function"; emit f subject file key(span);'
name	file	span	line	column
ready	app/src/lib.rs	11..50	3	1
double	app/src/util.rs	0..48	1	1
ready	support/src/lib.rs	0..17	1	1
ready	tool/src/lib.rs	0..17	1	1

```

Give `--text` more than once to run several queries over one read of the repository. The answer starts with a `rule` column, the query's place among them, and has every query's columns, empty where a query has none (here as JSON, with `--json`):

```console
$ ryo query run --root languages/rust/tests/fixtures/manifests/packages --text 'let f = select name, file, span from "function"; emit f subject file key(span);' --text 'let d = select source, target, kind from "package-dependency"; emit d subject source key(target);' --json
{"columns":["rule","name","file","span","source","target","kind","line","column"],"rows":[["1","ready","app/src/lib.rs",{"start":11,"end":50},null,null,null,3,1],["1","double","app/src/util.rs",{"start":0,"end":48},null,null,null,1,1],["1","ready","support/src/lib.rs",{"start":0,"end":17},null,null,null,1,1],["1","ready","tool/src/lib.rs",{"start":0,"end":17},null,null,null,1,1],["2",null,null,null,"app/Cargo.toml",null,"normal",null,null],["2",null,null,null,"app/Cargo.toml","support/Cargo.toml","normal",null,null],["2",null,null,null,"app/Cargo.toml","tool/Cargo.toml","build",null,null],["2",null,null,null,"app/Cargo.toml","tool/Cargo.toml","dev",null,null]],"count":8,"shown":8,"unread":[]}

```

`ryo relation list` prints the relations a query reads, and `ryo relation describe` the columns of one and what each language reads of it:

```console
$ ryo relation list
analyzed-source	A source file whose written paths a language examined in full.
annotates	The type a binding, a parameter or a function result is annotated with, as written.
...

$ ryo relation describe defines
defines: A name a file declares or brings in, under its shared kind, with the language's own refinement.
  id: node
  kind: name
  refinement: name
  name: name
  owner: node
  file: file
  span: span
  css: not in this language (no-such-construct)
  go: reads
  html: reads
  javascript: reads
  json: not in this language (not-code)
  markdown: reads
  python: reads
  rust: reads
  toml: not in this language (not-code)
  typescript: reads

```

RyoQL itself is described in [the RyoQL reference](https://github.com/idiomatic-sh/idiomatic/blob/main/crates/ryo/docs/ryoql.md).
