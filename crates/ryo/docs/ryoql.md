# RyoQL

The reference of RyoQL, the query language of Ryo, which Tastemaker's rules are written in.

RyoQL asks questions of code. Ryo reads every file into relations: tables of facts, such as each function a file declares or each import it writes. A query picks rows from those relations, narrows them, joins them and emits the rows that are findings. A relation has the same name and the same columns in every language, so one query reads Rust, Python and TypeScript alike. `ryo relation list`, `ryo relation describe <name>`, `ryo operator list` and `ryo operator describe <name>` print the catalog this page tables; `ryo query validate --text '<query>'` checks a query without running it.

## A first query

Every comment that holds `TODO` is a finding:

```text
let found = select * from "comment" where glob_any(text, "*TODO*");
emit found subject "file" key("span");
```

`let` names a step, `select * from "comment"` reads every row of the `comment` relation, `where` keeps the rows whose text matches the glob, and `emit` turns each row left into a finding at the file and span the row names.

## Statements

A query is a list of statements, each ending in `;`, with exactly one `emit`:

| Statement | Written | Does |
| --- | --- | --- |
| `import` | `import suppress from "python/ruff/suppress";` | Makes a named module available to calls in this query. |
| `param` | `param max-lines: integer;` | Takes a value from outside the query: from the entry that places the rule in Tastemaker, or from the SDK caller. Used as `$max-lines`. |
| `let` | `let found = <expression>;` | Names a step: a `select`, or a call of an operator. A later step reads it by name. |
| `emit` | `emit found subject "file" key("span");` | Turns each row of a step into a finding: `subject` is the column naming the file, `key` the columns that place the finding in it. |

A `param` has one of these types: `boolean`, `integer`, `string`, `path`, `glob`, `list<…>` and `set<…>` of those (as `set<string>`), and `list<table>`, read by the `rows` operator.

## Reusable query fragments

An SDK query may include named, immutable RyoQL modules. The root imports a module by its logical ID and calls the imported name like an operator:

```text
import suppress from "python/ruff/suppress";
let violations = suppress(candidates=locations, inline-pattern="...E722...", range-pattern="...");
emit violations subject "file" key("span");
```

A module declares relation inputs, scalar parameters, its output relation shape, and the step it returns:

```text
input candidates: relation<id: node, file: file, span: span, line: number, code: text>;
param inline-pattern: string;
param range-pattern: string;
output relation<id: node, file: file, span: span>;
let inline = select id, file, span from candidates where matches(code, $inline-pattern);
let ranges = select id, file, span from candidates where matches(code, $range-pattern);
let violations = union(left=inline, right=ranges);
return violations;
```

Imports may be nested, but cycles, missing modules, duplicate argument names, and incompatible input or output shapes are rejected. Each call expands with its own step identities; local names in one invocation cannot capture names in another. Module IDs and exact source text are supplied in the SDK query bundle, not loaded by the compiler from disk.

## Expressions

A step is a `select` or an operator call:

| Form | Example |
| --- | --- |
| Every column | `select * from "function"` |
| Named columns, renamed | `select l.name as "name", l.file as "file" from "function" as l` |
| A join | `select l.file as "file" from "function" as l join "comment" as r on l.file = r.file` |
| A filter | `select * from "defines" where kind = "class" and not glob_any(name, "*Service")` |
| An operator call | `project(from=found, columns=["path"])` |

A source in quotes is a relation; a bare word is an earlier step. `as` gives a source an alias; after a join, `l.name` is the column `name` of the side `l`, kept in the output or not. A join takes one or more `on` keys joined with `and`.

## Filters

A `where` holds comparisons joined with `and` and `or`; `and` binds tighter than `or`, and parentheses group.

| Filter | Example | Keeps a row when |
| --- | --- | --- |
| `=`, `!=` | `kind = "class"` | the two sides are equal, or differ |
| `<`, `<=`, `>`, `>=` | `lines > $max-lines` | the numbers, or the texts, compare so |
| `in`, `not in` | `kind in $kinds` | the value is, or is not, in a `list<…>` or `set<…>` parameter |
| `glob_any`, `not glob_any` | `glob_any(target, $forbidden)` | the text matches the glob, or any glob of a parameter, or none |
| `starts_with`, `ends_with` | `ends_with(name, "Service")` | the first text starts, or ends, with the second |
| `float_text_gte` | `float_text_gte(text, $limit)` | the first text, read as a number, is at least the second |

`in` and `not in` compare against a parameter; a list written in the query, as in `kind in ["class", "function"]`, is not RyoQL.

A side of a comparison is a column, a text in quotes, a number, a `$parameter` or a text function:

| Function | Example | Gives |
| --- | --- | --- |
| `concat` | `concat(pascal(pkg), "Service")` | its two or more texts joined |
| `pascal` | `pascal("todo_lists")` | `TodoLists` |
| `camel` | `camel("todo_lists")` | `todoLists` |
| `snake` | `snake("TodoLists")` | `todo_lists` |
| `kebab` | `kebab("TodoLists")` | `todo-lists` |

## Operators

`select` is written as the expressions above show; every other operator is a call whose arguments are written `name=value`, a list in brackets and a step or a relation by its name:

| Operator | Arguments | Does |
| --- | --- | --- |
| `select` | `select … from … where …` | Keeps the rows for which every comparison is true. |
| `project` | `from`, `columns` | Keeps the named columns and drops equal rows. |
| `extend` | `from`, `values` | Appends number or flag columns, from numbers or parameters. |
| `span-between` | `from`, `start`, `end`, `output` | Appends `[start.start, end.end)` from two span columns in per-file rows. |
| `join` | `left`, `right`, `left-on`, `right-on`, `output` | Combines rows with equal keys; empty keys give every pair. |
| `union` | `left`, `right` | The rows of either input, both of the same columns. |
| `intersection` | `left`, `right` | The rows of both inputs. |
| `difference` | `left`, `right` | The rows of the left input the right one lacks. |
| `antijoin` | `left`, `right`, `left-on`, `right-on` | The left rows no right row matches on the keys. |
| `group-count` | `from`, `keys`, `count`, `distinct` | Groups rows by `keys` and appends how many, or how many distinct values of a column. |
| `group-digest` | `from`, `keys`, `distinct`, `digest`, `count` | Groups rows and appends a digest of the distinct values of a column: equal sets give equal digests. |
| `capture` | `from`, `path`, `pattern`, `captures` | Matches a path column to a pattern such as `src/{*dir}/{name}.py` and appends the named parts. |
| `words` | `from`, `text`, `word` | Splits a text into lower case words, one row each. |
| `rank` | `from`, `column`, `sets`, `output` | Appends the place of the first set of a `list<set<string>>` parameter that holds the text; a row no set holds is dropped. |
| `axis` | `from`, `relation`, `direction`, `domain` | Walks a hierarchy, such as `syntax-child`, and appends the related entity. |
| `rows` | `from`, `columns` | The rows of a `list<table>` parameter, one for each table. |
| `closure` | `edges`, `source`, `destination` | The pairs one entity reaches from another, transitively. |
| `cycles` | `edges`, `source`, `destination` | One row for each entity that can reach itself, with the entity in both columns: two files that import each other give two rows, `a.js` and `b.js`. |

## Relations

Each column has a type: `node` is an identifier that joins with the `node` columns of other relations, `name` is text, `file` is a path from the root, `span` is a place in a file, `number` and `flag` are what they say. The key columns say what one row is.

| Relation | One row per | Key columns | Other columns |
| --- | --- | --- | --- |
| `analyzed-source` | source file a language examined in full | `path` | |
| `annotates` | type annotation of a binding, a parameter or a result | `occurrence` | `owner`, `annotation`, `file`, `span` |
| `assigns` | string literal bound to a name | `occurrence` | `binding`, `value`, `owner`, `file`, `span` |
| `call-argument` | argument of a call of a path written out | `expression` | `call`, `position`, `passes`, `file`, `span` |
| `comment` | line, block or doc comment | `id` | `text`, `file`, `span`, `doc` |
| `decorates` | decoration of a declaration | `occurrence` | `declaration`, `ordinal`, `target`, `arguments`, `file`, `span` |
| `defines` | name a file declares or brings in | `id` | `kind`, `refinement`, `name`, `owner`, `file`, `span` |
| `directive` | instruction to the build tool written in source | `file`, `span` | `kind`, `text` |
| `entry` | key of a data document, with its value | `occurrence` | `decoded`, `key`, `kind`, `ordinal`, `owner`, `file`, `span`, `text`, `boolean`, `integer`, `float`, `datetime`, `radix` |
| `file-package` | file of the repository, with its package | `path` | `package` |
| `filesystem.file` | file of the repository | `path` | |
| `filesystem.line-count` | file of the repository, with its line count | `path` | `lines` |
| `function` | function declaration | `id` | `name`, `file`, `span` |
| `inherits` | base a type extends or implements | `occurrence` | `declaration`, `target`, `file`, `span` |
| `mapping` | table of keys of a data document | `id` | `kind`, `ordinal`, `owner`, `file` |
| `mention` | identifier inside a macro call's tokens | `occurrence` | `owner`, `target`, `file`, `span` |
| `package-dependency` | dependency of one package on another | `source`, `target` | `binding`, `package`, `kind`, `condition`, `optional`, `origin`, `requirement`, `exact`, `branch`, `tag`, `revision`, `interpreter`, `catalog`, `replacement` |
| `parameter` | parameter of a function, method or closure | `id` | `kind`, `refinement`, `name`, `ordinal`, `owner`, `file`, `span` |
| `prose` | run of text a reader of a document reads | `id` | `owner`, `text`, `file`, `span` |
| `reference` | name used where it is not declared | `occurrence` | `owner`, `target`, `file`, `span` |
| `resolves` | type name written in a type position, with the declaration or import that binds it in the file | `occurrence` | `owner`, `name`, `definition`, `file`, `span` |
| `scope` | scope, with the scope that encloses it | `parent`, `child` | `file`, `span` |
| `sequence-element` | position of a sequence of a data document | `occurrence` | `decoded`, `kind`, `ordinal`, `owner`, `file`, `span`, `text`, `boolean`, `integer`, `float`, `datetime`, `radix` |
| `source-dependency` | path one source file writes to what another owns | `source`, `target`, `span` | `written` |
| `static-call` | call of a path written out | `occurrence` | `owner`, `target`, `file`, `span` |
| `static-import` | name an import brings into scope | `occurrence` | `owner`, `binding`, `target`, `file`, `span` |
| `string-literal` | string literal, decoded | `occurrence` | `owner`, `value`, `file`, `span`, `import` |
| `syntax-child` | edge of the syntax tree | `parent`, `child` | `ordinal`, `field`, `file` |
| `syntax-error` | diagnostic of the parser | `id` | `message`, `file`, `span` |
| `syntax-tree` | node or token of the syntax tree | `id` | `kind`, `text`, `file`, `span` |
| `visibility` | visibility a declaration is written with | `occurrence` | `declaration`, `text`, `file`, `span` |
| `workspace-member` | package that belongs to a workspace | `workspace`, `package` | |

`call-argument.call` is the `occurrence` of the argument's `static-call` row, `position` is the place of the parameter the argument is bound to, from zero, as the call's syntax says it (in Rust the receiver of a method call is position 0, its `self`; in the other languages it is the place among the arguments in the parentheses), and `passes` is the name the argument hands on unchanged (behind a reference, a dereference or a spread) or the empty text. `resolves.definition` is the `id` of the `defines` row that binds the name where it is written: a declaration of the file, the import that brings the name in, or a type parameter; a name no row binds, a qualified name and the types of the language's library have no row.

`filesystem.file` and `filesystem.line-count` come from the engine for every file; each other relation is read by the languages `ryo relation describe <name>` lists. A host such as Tastemaker adds relations of its own (`tastemaker.placed`).

## Declaration kinds

Some columns hold a closed set of words, and a query that compares one with another word is refused when it is compiled, naming the words there are:

| Column | Words |
| --- | --- |
| `defines.kind` | `function`, `method`, `class`, `struct`, `enum`, `enum-member`, `trait`, `interface`, `type`, `type-alias`, `type-parameter`, `namespace`, `package`, `import`, `variable`, `constant`, `parameter`, `property`, `macro`, `section`, `label`, `implementation` |
| `parameter.kind` | `receiver`, `positional`, `positional-or-keyword`, `keyword-only`, `variadic-positional`, `variadic-keyword`, `result` |
| `entry.kind`, `sequence-element.kind` | `string`, `integer`, `float`, `boolean`, `datetime`, `sequence`, `inline-mapping`, `mapping`, `sequence-of-mappings`, `null` |
| `mapping.kind` | `standard`, `implicit`, `dotted`, `inline`, `sequence-member` |
| `package-dependency.kind` | `normal`, `dev`, `build`, `peer` |
| `package-dependency.origin` | `registry`, `path`, `git`, `url`, `workspace` |

`defines.refinement` holds the language's own word for the declaration, such as `async-function` in Python. A flag column is compared with `"true"` or `"false"`, as in `doc = "true"` on `comment`.

## Worked example: a parameter

No file imports what the entry forbids; the entry passes `forbidden = ["*.routers.*"]`:

```text
param forbidden: set<string>;
let found = select * from "static-import" where glob_any(target, $forbidden);
emit found subject "file" key("span");
```

## Worked example: a join

Each `src/services/<pkg>/service.py` declares a class named for its package, `todo_lists` as `TodoListsService`. `capture` reads the package from the path, the join pairs each service file with the classes it declares, and `antijoin` keeps the files that have none of the right name:

```text
let services = capture(from="filesystem.file", path="path", pattern="src/services/{pkg}/service.py", captures=["pkg"]);
let classes = select * from "defines" where kind = "class";
let named = select c."file" as "file", p."pkg" as "pkg" from classes as c join services as p on c."file" = p."path" where c.name = concat(pascal(p.pkg), "Service");
let missing = antijoin(left=services, right=named, left-on=["path"], right-on=["file"]);
emit missing subject "path" key("path");
```

## Worked example: a count

No file declares more than 20 functions:

```text
let functions = select * from "function";
let per-file = group-count(from=functions, keys=["file"], count="functions");
let crowded = select * from per-file where functions > 20;
emit crowded subject "file" key("file");
```

## Literal replacement and ordered matching

Python's optional `raw-format-segment` relation records one parser outcome per
ordinary string token, retaining the enclosing expression, token identity and source
order. Prefixes and quotes are excluded from the parser input; escapes and line
endings remain unchanged. Each segment is parsed independently, including malformed
and empty bodies. `raw-format-field` records conversions from successful segments
with both segment-local and expression-wide order. A malformed segment contributes
no fields, including any otherwise valid prefix. Unavailable source knowledge makes
the relation partial and prevents precise flattened fields for the affected expression.
These relations have independent demand and caching from decoded `format-parse`,
`format-field` and brace-format relations.

`span-between(from=rows, start="literal_span", end="expression_span", output="diagnostic_span")`
appends a typed span while retaining every column and repeated row. It uses the first
span's start and the second span's end, including when the expression starts before
the literal. Overlap, containment, gaps, empty spans and EOF offsets are valid. Offsets
are bytes; no UTF-8 boundary restriction applies. Each located input must satisfy
`start <= end <= source length`, and the composed start must not exceed its end.
Invalid ranges fail execution. An absent input produces an absent output while the
other located input is still validated. Construction requires per-file input, also
inside an instantiated fragment; the result may subsequently cross the file frontier.
The output name must be new.

`replace(from=rows, text="label", pattern="\s+", replacement="", output="compact")`
appends a text column after replacing every non-overlapping regex match in the input
column. The pattern and replacement are constant text. Replacement text is literal:
`$1` and backslashes have no substitution meaning. The same Unicode regex grammar
as `matches` applies; invalid patterns fail compilation even for an empty input.
Zero-width matches follow the regex engine's forward-progress rules. Input rows and
columns are retained, including rows whose text has no match. The output column
must have a new name, and the input column must hold text.

`ordered-match` pairs rows within equal typed key groups. It visits each left row
in ascending numeric order and consumes the earliest still-unmatched right row
whose numeric order is strictly greater. Earlier or equal right rows are discarded;
an unmatched right row never consumes a future left row. Unmatched left rows have
no output; an `antijoin` can recover them. Input iteration order has no effect.
Order values must be unique within each side and key group; duplicates are errors,
including groups having no counterpart. Key lists must have equal lengths and
matching types; an empty list means one group. Output names and explicit sources
use the existing `join` map syntax, so same-named input columns are unambiguous:

```text
let pairs = ordered-match(left=requests, right=replies,
    left-on=["channel"], right-on=["channel"],
    left-order="sequence", right-order="sequence",
    output={request: [left, "id"], reply: [right, "id"]});
```

For left orders `[2, 3]` and right orders `[1, 4, 5]`, the pairs are `(2, 4)` and
`(3, 5)`. For left `[2, 6]` and right `[3, 4, 7]`, the pairs are `(2, 3)` and
`(6, 7)`. These rules apply independently to each key group. Output row order is
not a query contract, as with other relational operators.
