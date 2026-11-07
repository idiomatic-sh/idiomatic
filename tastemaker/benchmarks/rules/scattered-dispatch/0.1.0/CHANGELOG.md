# scattered-dispatch 0.1.0

The first release. The principle: colocation is by
domain, not primitive. An enum whose variants other domains match in many files has its
behaviour scattered across them (Tastemaker's `DefinitionKind`, matched across six crates;
the provider feature dispositions), so adding a variant is a change in each file.

An enum declared in scope is reported at its declaration when match-arm patterns name its
variants (`Enum::Variant`, alone, nested, in an or-pattern, as a tuple or record variant,
or fully qualified) in three or more files outside the directory of the file declaring it.
There is one finding for each of those files.

## Placing it

The shared rule block, over the whole product, with tests, benches and test data left out:

```toml
{ package = "idiomatic.arch", version = "0.1.0", rule = "scattered-dispatch", include = ["**"],
 exclude = ["**/tests/**", "**/benches/**", "**/*_test.rs", "**/fixtures.rs"] }
```

Tests and test data enumerate an enum's cases by design, as the boundary rules
(`io-at-boundary`) leave them out too.

## Limits

Name matching is by the written path's text; semantic resolution is not done.

- An arm `Kind::Spec` counts for every enum named `Kind`, so a name two or more enums
 declare in scope is ambiguous and never reported.
- A variant imported bare (`Spec =>`) or reached through `Self::` is not seen.
- Only match arms count: `if let`, `let else`, a `for` pattern and a variant built in an
 expression do not.
- The enum's own directory is the directory of its declaring file, with everything under
 it; for a hex's `types.rs` that is the domain. An enum declared at the root owns
 everything.
- Paths of any depth are read: a capture answers a row for each directory of the
 using file's path, and a use is inside when one of them is the enum's own directory.

## Fixtures

`scattered` and `deep-outside` fail, and so does each pattern form alone in all three
files (`unit-variants`, `tuple-variants`, `record-variants`, `nested-variants`, `or-variants`,
`qualified-variants`); `two-files`, `inside-domain`, `ambiguous`, `not-arms`, `for-in-arm`,
`root-declared`, `bare-and-self`, `tests-excluded` and `deep-inside` pass.

Fixtures compare findings by rule and path only, so they cannot show one finding per file
or the level.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.
