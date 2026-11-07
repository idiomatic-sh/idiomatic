# Tastemaker

Tastemaker is a linter built to codify and enforce architectural standards. It has no preset rules or configurations. All behavior is authored by you.

What's special about Tastemaker is that it is able to compare files and their contents. This allows for repeatable and consistent design patterns.

An example. Say you have a todo API in Python with this target directory structure:

```text
.
├── src
│   ├── models
│   │   └── <model>.py
│   ├── routers
│   │   └── <router>
│   │       └── routes.py
│   ├── services
│   │   └── <service>
│   │       └── service.py
│   └── main.py
└── tests
    ├── models
    │   └── test_<model>.py
    ├── routers
    │   └── <router>
    │       └── test_routes.py
    ├── services
    │   └── <service>
    │       └── test_service.py
    └── test_main.py
```

In order to have your team follow this structure reliably, you may want:

| Type | Rule | Primitive |
| --- | --- | --- |
| Lock directory structure | No top-level directories except those declared above | rule |
| Consistent file names | Each package in `services/<service>/` contains `service.py` | spec + template |
| Naming convention | `service.py` contains a class named `<Name>Service` | rule |
| Import direction | `service.py` may not import from `routers/` | rule |
| Required tests | Every file in `src/` has a `test_*.py` in `tests/` | rule |
| Framework conventions | Every route in `routes.py` is `async def` | rule |
| Complexity limits | No function is over 60 lines | rule |
| Dependency graph | No import cycle between the packages under `services/` | rule |
| Banned patterns | No `TODO` comments | rule |
| Duplicate code | No two functions spell the same body | rule |
| Idiomatic code | `__all__` lists only public names, and lists every public top-level name | rule |

The whole example, with one planted violation per row, is in [examples/python/api](https://github.com/idiomatic-sh/idiomatic/tree/main/crates/tastemaker/examples/python/api).

Tastemaker codifies these decisions by allowing you to articulate rules like these. It also provides a scaffold engine so directories and files are reliably created as defined.

There are four design primitives to build with:

| Primitive | Description |
| --- | --- |
| rule | Lint rule. |
| template | File pre-populated with code, meant to be filled in for each instance. |
| spec | Set of rules and templates combined to describe and enforce a design pattern. |
| tastemaker.toml | Composition of rules, templates, and specs. |

Together, these can enforce directory structure, design patterns, naming conventions, stylistic rules, and more.

## Install

| Language | Command |
| --- | --- |
| typescript | `pnpm i -D @idiomatic-sh/tastemaker` |
| python | `uv add --dev idiomatic-sh-tastemaker` |
| rust | `cargo install --locked idiomatic-sh-tastemaker` |

The command is `tm`. In a repository that declares its tools, `uv run --script tools/install.py` installs every tool at the version the repository pins, `tm` included.

## Usage

### Set up

```console
$ tm init
wrote tastemaker/package.toml
wrote tastemaker/templates/unchecked/0.1.0/template.toml
wrote tastemaker.lock
wrote tastemaker.toml
next: tm explain          what applies here, and every topic
next: tm explain --help   how to write a rule, spec or template
next: tm check            the findings

```

`init` writes a `tastemaker.toml` and a local package in `tastemaker/`, where your own rules, templates and specs live.

### Author a rule

```console
$ tm create rule/example-rule --set extension=rs
wrote tastemaker/rules/example-rule/0.1.0/rule.toml
next: write a failing and a passing [[scenarios]], then the query (`tm explain rule`), then run `tm validate rule/example-rule`

```

A rule is one file, `tastemaker/rules/example-rule/0.1.0/rule.toml`: a RyoQL query, the message a finding carries, and the scenarios that prove it.

```toml
description = "What the rule checks, on one line."
why         = "Why it matters, and what a reader does about a finding."

query = '''
let found = select * from "comment" where text = "";
emit found subject "file" key("span");
'''

[diagnostic]
primary = "span"
message = "what is wrong here"
help    = "What to change, and where; and whether an ignore with a reason is ever right."

[[scenarios]]
id       = "empty-line-comment"
scenario = "An empty line comment is reported."
given.include = ["**"]
given.files."src/lib.rs" = "//\npub fn value() -> u8 {\n    1\n}\n"
then.findings = [{ path = "src/lib.rs", line = 1 }]

[[scenarios]]
id       = "line-comment-with-text"
scenario = "A line comment that says something is not reported."
given.include = ["**"]
given.files."src/lib.rs" = "// the value\npub fn value() -> u8 {\n    1\n}\n"
```

A scenario names the files it starts from and the findings expected; a scenario with no findings is a pass. `tm validate` runs every scenario.

### RyoQL

A rule's `query` is written in RyoQL. Ryo reads your code into relations: tables of facts such as `defines` (every name a file declares), `static-import`, `function`, `comment` and `filesystem.file`. A relation has the same name and columns in every language. `tm list relations` lists them, and `tm explain relation/<name>` shows its columns and which languages read it.

A query is a few lines: `param` takes a value from the entry in `tastemaker.toml`, each `let` builds a step from a relation or an earlier step (a `select`, or an operator such as `join`, `antijoin`, `capture` or `group-count`), and exactly one `emit` turns each row of a step into a finding. `tm explain ryoql` walks through writing one, and the [RyoQL reference](https://github.com/idiomatic-sh/idiomatic/blob/main/crates/ryo/docs/ryoql.md) tables every relation, statement, filter, text function and operator, with worked examples.

For example, every route in `routes.py` must be `async def`. The Python reader records each function's `refinement`, `function` or `async-function`:

```toml
query = '''
let found = select * from "defines" where kind = "function" and refinement != "async-function";
emit found subject "file" key("span");
'''
```

Placed with `include = ["**/routes.py"]`, `tm check` reports:

```text
error[local/async-routes]: src/routers/todos/routes.py:33:5: this route is not `async def`
  to fix: Write the route as `async def`.
```

### Create a template

```console
$ tm create template/example-template --set extension=rs
wrote tastemaker/templates/example-template/0.1.0/template.toml
next: write its content, then its [[scenarios]] (`tm explain template`), then run `tm validate template/example-template`

```

A template is a file's content and how strictly a placed file must match it: `match = "contains"` (the default, the file may hold more), `"declarations"` (the same declarations, bodies free) or `"text"` (byte for byte). `tm scaffold` writes a placed file that does not exist yet.

### Compose a spec

```console
$ tm create spec/example-spec
wrote tastemaker/specs/example-spec/0.1.0/spec.toml
next: write its [[scenarios]], then its entries (`tm explain spec`), then run `tm validate spec/example-spec`

```

A spec describes a kind of directory: the templates and rules every directory of that kind holds.

### Place them

```console
$ tm add example-rule --at src
added local/example-rule@0.1.0 to tastemaker.toml
next: tm check

$ tm add example-template --at src/example.rs
added local/example-template@0.1.0 to tastemaker.toml
next: tm scaffold example-template --at src/example.rs

```

Each `add` writes an entry into `tastemaker.toml`, and `tastemaker.lock` pins the version it uses:

```toml
rules = [{ rule = "local/example-rule", include = ["src"], description = "..." }]
templates = [{ template = "local/example-template", file = "src/example.rs", description = "..." }]
```

### Run

```console
$ tm validate
4 version(s) hold, 0 do not; 0 proof(s) kept from an earlier run

$ tm check --fix
? 1
error[local/example-rule#coverage]: tastemaker.toml: `local/example-rule` reaches no file: its include matches nothing here
error[local/example-template#count]: tastemaker.toml: `src/example.rs` matches 0 file(s) at the project's root, but `local/example-template` requires exactly 1
local: 2 error(s), 0 suppressed

```

- `tm check`: every finding in the workspace, proving first every rule, template and spec as `tm validate` does; `tm check --fix` after editing tastemaker.toml
- `tm validate`: prove every rule, template and spec by its scenarios, placed or not
- `tm scaffold <entry> --at <path>`: write the files an entry places that do not exist yet
- `tm update`: move the versions the lock pins to the latest their package offers; in a package of your own workspace the version it leaves unpinned is then read by no one, so `tm validate` and `tm check` name it and stop (exit 2) until you delete its directory (git keeps it, and a workspace that needs it again pins it)
- `tm explain`: what applies here; `tm explain <path>` or `tm explain rule` for one
- `tm list`: the definitions; `tm list relations` and `tm list operators` for RyoQL

`tm check --format json` prints `workspace`, `findings`, `suppressed` and `rules`: `rules` says what to do once for each rule (`"rules": { "p/r": "fix it" }`), and a finding carries its own `help` only when it says something other than its rule's; a rule whose findings say different things (a help that names what was found) has no entry, and each finding says its own.

`tm check --format sarif` prints the findings for [Asymptote](https://github.com/idiomatic-sh/idiomatic/blob/main/crates/asymptote/README.md) to ratchet.
