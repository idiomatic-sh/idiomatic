# readers-in-items

## 0.2.0

The homes of a reader are a param (`homes`, path patterns) the language spec passes, not a path written in the query. A reader in a home passes; a reader anywhere else, including a concept's service beside its home, still fails. Fixtures: `in-a-parse-reader`, `reader-in-a-reader-concept-service`, `reader-in-the-parse-contract`.

## 0.1.0

The first version. Placed over `languages/*/src` by ryo's tastemaker.toml.

A hand-written marker impl (`declared::Syntax`, `declared::Manifest`) is a finding wherever it stands: the seal is a hidden public module the macros need to reach, so it stops an impl that forgets the marker, not a deliberate one. Fixture: `marker-by-hand`.
