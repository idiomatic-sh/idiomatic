# toml standard

## Source

`toml.abnf`, the ABNF grammar of TOML that the TOML project publishes (github.com/toml-lang/toml, MIT License, Copyright (c) Tom Preston-Werner), copied unchanged. Its rules name the nodes and its terminal rules the tokens of the language's trees.

`<path>.tree` beside the files of `corpus/`, `suite/`, `relations/` and `allocation/` (as `standard/corpus/<file>.tree`, `standard/suite/valid/<path>.toml.tree`, `standard/relations/<relation>/input.toml.tree`): what an ABNF engine, run over `toml.abnf`, printed as the parse tree of that file, recorded once and never edited. Every `.toml` file of those four directories and the corpus's `Cargo.lock` has one: 733 files, the 218 valid and the 494 invalid of toml-test among them. The adapter's tree of each must be the standard's (`adapter_test.rs` of the adapter).

## Dumps

The engine is the Python package `abnf` 2.9.0 (`uv run --with abnf==2.9.0 python dump.py <crate>/tests/fixtures`, CPython `3.15.0rc2 (main, Sep  1 2026, 14:16:41) [Clang 22.1.3 ]`). It parses the whole file with the rule `toml`; the script that printed the dumps is below. Its three differences from printing the engine's tree as it is:

- the three core rules the grammar reproduces at its end (`ALPHA`, `DIGIT`, `HEXDIG`) are left out of the grammar the engine loads, since the engine has them built in as RFC 5234 defines them;
- a line is a rule, indented one more than the rule it is a part of, and the bytes it matched, `<rule> <start>-<end>`, in UTF-8 bytes and not in the characters the engine counts; a rule that matched nothing and the literal characters of a rule are left out (the root is always there);
- the engine's tree stops at a rule that is a token of the language (a string, a number, a key, a comment, a newline, `ws`, `wschar`), whose characters it has no use for.

A file the engine does not parse is `refused: ...` and no tree: the standard has no tree for a document that is not UTF-8 or not TOML. Of the 494 invalid files of toml-test, 395 are refused by the grammar; the other 99 are invalid by the meaning the text of the standard gives them (a duplicate key, a table defined twice), which the grammar does not say, and have the grammar's tree. The test holds the tree of those to it as of any file, and a file the standard refuses to a tree of ours that holds text it could not read or a diagnostic.

```python
import sys, pathlib
from abnf import Rule, ParseError

CORE = {"ALPHA", "DIGIT", "HEXDIG"}
STOP = {"unquoted-key", "basic-string", "literal-string", "ml-basic-string", "ml-literal-string",
        "integer", "float", "boolean", "date-time", "comment", "wschar", "newline", "ws"}

class Toml(Rule):
    pass

def load(path):
    kept = [line for line in pathlib.Path(path).read_text().splitlines(keepends=True)
            if line.split("=")[0].strip() not in CORE]
    Toml.load_grammar("".join(kept))
    return Toml("toml")

def lines(node, at, depth, text_bytes_offsets, out):
    end = at + len(node.value)
    if node.name == "literal" or (depth and not node.value):
        return
    out.append(f"{' ' * depth}{node.name} {text_bytes_offsets[at]}-{text_bytes_offsets[end]}")
    if node.name not in STOP:
        cursor = at
        for child in node.children:
            lines(child, cursor, depth + 1, text_bytes_offsets, out)
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
    rule = load(fixtures / "standard" / "toml.abnf")
    inputs = [path for folder in ("corpus", "suite", "relations", "allocation")
              for path in sorted((fixtures / folder).rglob("*"))
              if path.is_file() and (path.suffix == ".toml" or path.name == "Cargo.lock")]
    for path in inputs:
        target = fixtures / "standard" / (str(path.relative_to(fixtures)) + ".tree")
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(dump(rule, path.read_bytes()))
```

## Version

TOML 1.1.0 (the tag `1.1.0` of the repository).
