# json standard

## Source

`json.abnf`, the ABNF grammar of JSON that RFC 8259 (STD 90, T. Bray, Ed., December 2017) gives in its sections 2 to 7, copied unchanged and left to be read by an ABNF engine. Its rules name the nodes and its terminal rules the tokens of the language's trees. The RFC is published under the IETF Trust's Legal Provisions (`NOTICE`).

`<path>.tree` beside the files of `corpus/`, `suite/`, `relations/` and `allocation/` (as `standard/corpus/<origin>/<file>.tree`, `standard/suite/valid/<file>.json.tree`, `standard/relations/<relation>/input.json.tree`): what an ABNF engine, run over `json.abnf`, printed as the parse tree of that file, recorded once and never edited. Every `.json` and `.jsonc` file of those four directories has one. The adapter's tree of each must be the standard's (`adapter_test.rs` of the adapter).

## Dumps

The engine is the Python package `abnf` 2.9.0 (`uv run --with abnf==2.9.0 python dump.py <crate>/tests/fixtures`). It parses the whole file with the rule `JSON-text`; the script that printed the dumps is below. Its differences from printing the engine's tree as it is:

- the grammar's rule `char` is loaded as `json-char`: RFC 5234 reserves `CHAR` as a core rule, and the engine refuses a grammar that redefines it (the rule is below a string, which the dump stops at);
- a line is a rule, indented one more than the rule it is a part of, and the bytes it matched, `<rule> <start>-<end>`, in UTF-8 bytes and not in the characters the engine counts; a rule that matched nothing and the literal characters of a rule are left out (the root is always there);
- the engine's tree stops at a rule that is a token of the language (a string, a number, a literal, `ws`), whose characters it has no use for.

A file the engine does not parse is `refused: ...` and no tree: the standard has no tree for a document that is not UTF-8 or not JSON. That is the standard's answer to every file with a comment or a comma after the last member, and to the 188 invalid files of the suite. A file the language reads as json with comments (`.jsonc`, `tsconfig.json`, `jsconfig.json`) may be refused by the standard and read clean by the language; a file it reads as json the standard defines must say it is refused, with text it could not read or a diagnostic.

```python
import re, sys, pathlib
from abnf import Rule, ParseError

STOP = {"json-char", "string", "number", "false", "null", "true", "ws"}

class Json(Rule):
    pass

def load(path):
    # RFC 5234 reserves the name of the core rule CHAR; the standard's own rule char is loaded as json-char.
    Json.load_grammar(re.sub(r"\bchar\b", "json-char", pathlib.Path(path).read_text()))
    return Json("json-text")

def lines(node, at, depth, offsets, out):
    end = at + len(node.value)
    if node.name == "literal" or (depth and not node.value):
        return
    out.append(f"{' ' * depth}{node.name.lower()} {offsets[at]}-{offsets[end]}")
    if node.name.lower() not in STOP:
        cursor = at
        for child in node.children:
            lines(child, cursor, depth + 1, offsets, out)
            cursor += len(child.value)

def dump(rule, data):
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        return "refused: not UTF-8\n"
    offsets = [0]
    for ch in text:
        offsets.append(offsets[-1] + len(ch.encode("utf-8")))
    try:
        tree = rule.parse_all(text)
    except ParseError as error:
        return f"refused: no parse at character {error.start}\n"
    out = []
    lines(tree, 0, 0, offsets, out)
    return "\n".join(out) + "\n"

if __name__ == "__main__":
    fixtures = pathlib.Path(sys.argv[1])
    rule = load(fixtures / "standard" / "json.abnf")
    inputs = [path for folder in ("corpus", "suite", "relations", "allocation")
              for path in sorted((fixtures / folder).rglob("*"))
              if path.is_file() and path.suffix in (".json", ".jsonc")]
    for path in inputs:
        target = fixtures / "standard" / (str(path.relative_to(fixtures)) + ".tree")
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(dump(rule, path.read_bytes()))
```

## Version

RFC 8259, December 2017. JSON with comments is not in it; the language reads it as the configuration files of TypeScript and VS Code do, and the differences section of the README says how.
