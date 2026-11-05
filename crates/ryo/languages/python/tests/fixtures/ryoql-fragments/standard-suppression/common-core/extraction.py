"""Pin copied predicates, original/mapped location roles, and the additional-line checkpoint."""

import argparse
import hashlib
import json
from pathlib import Path
import re


def bindings(text):
    return dict(re.findall(r"^let (\w+) = (.*);$", text, re.MULTILINE))


def check(bench):
    fixture = Path(__file__).resolve().parent
    before = (fixture / "baseline-72ea.ryoql").read_text()
    assert (
        hashlib.sha256(before.encode()).hexdigest()
        == "72eaeed41d5a9a6393d5048cb7bf7389479a9ce622f4748db92f980e724a8b80"
    )
    after = {
        name: (bench / "fragments" / (name + ".ryoql")).read_text()
        for name in (
            "suppress",
            "suppress-directives",
            "suppress-unmapped",
            "suppress-with-additional-lines",
        )
    }
    old = bindings(before)
    core = bindings(after["suppress-directives"])
    wrapper = bindings(after["suppress"])
    mapping = bindings(
        before[before.index("let quoted =") : before.index("let comments =")]
    )
    for name, expression in mapping.items():
        assert wrapper[name] == expression, ("standard mapping changed", name)
        assert bindings(after["suppress-with-additional-lines"])[name] == expression, (
            "additional-line primary mapping changed",
            name,
        )
    copied = set(old) - set(mapping) - {"located", "unsuppressed_files"}
    for name in copied:
        assert core[name] == old[name], ("directive predicate/producer changed", name)
    assert core["located"] == "select id, file, span, line from locations"
    assert core["supp_original_locations"] == old["supp_original_locations"]
    assert (
        core["unsuppressed_additional"]
        == 'antijoin(left=unsuppressed_lines, right=additional_hidden_candidates, left-on=["id"], right-on=["id"])'
    )
    assert (
        core["additional_hidden_candidates"]
        == "select l.id as id from additional_locations as l join line_suppressions as r on l.file = r.file and l.line = r.line"
    )
    assert core["unsuppressed_files"] == old["unsuppressed_files"].replace(
        "left=unsuppressed_lines", "left=unsuppressed_additional"
    )
    assert (
        "locations=located, additional_locations=empty_additional"
        in wrapper["violations"]
    )
    unmapped = bindings(after["suppress-unmapped"])
    assert (
        "locations=candidate_locations, additional_locations=empty_additional"
        in unmapped["violations"]
    )
    assert not (set(unmapped) & set(mapping)), (
        "unmapped wrapper acquired a mapping graph"
    )
    for name in ("suppress", "suppress-unmapped"):
        assert (
            bindings(after[name])["empty_additional"]
            == "select id, file, line from candidate_locations where id != id"
        )
    receipt = json.loads((fixture / "source-receipt.json").read_text())
    for row in receipt["rows"]:
        query = (bench / row["path"].split("linters/ruff/", 1)[1]).read_text()
        assert hashlib.sha256(query.encode()).hexdigest() == row["after_sha256"]
        prelude = query.split("\n", 1)[1].split("let violations = suppress(", 1)[0]
        assert (
            hashlib.sha256(prelude.encode()).hexdigest()
            == row["preserved_prelude_sha256"]
        )
        assert row["inline_pattern"] in query and row["range_pattern"] in query
        assert query.splitlines()[-1] == row["emit"]
    # UP009's original alias included file but the consumed range projection read
    # only SourceLocation columns. Its code-token predicate was exactly standard.
    up009 = (bench / "rules/UP009/candidate.ryoql").read_text()
    preamble = bindings(up009)["preamble_code_tokens"]
    assert preamble == core["code_tokens"]
    return {
        "copied_bindings": len(copied),
        "unchanged_mapping_bindings": len(mapping),
        "original_range_location_producer_preserved": True,
        "unmapped_graph_absent": True,
        "additional_line_checkpoint": "after primary inline suppression, before standalone file suppression",
        "up009_alias_predicate_exact": True,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bench", type=Path)
    print(json.dumps(check(parser.parse_args().bench), indent=2))
