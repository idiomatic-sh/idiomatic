# no-json-in-process 0.3.0

The package layer. A package that depends on `serde_json` as a normal dependency is a
finding at that dependency's line in its manifest, unless it is a boundary package: the
package that holds the CLI spec's placed output module, or one whose manifest the entry's
`exclude` leaves out. `package-dependency` now has a row for every declared dependency,
`serde_json = "1"` among them, so the rule no longer sees sources only. The dependency is
matched by the package depended on, so a crate renamed in the manifest
(`json = { package = "serde_json" }`) counts, which 0.2.0 did not see.

## Moving up from 0.2.0

Set `version = "0.3.0"` on the entry and fix what it finds: move the `serde_json`
dependency of a package that holds no boundary into the adapter package that crosses the
boundary, or add that boundary package's manifest to the entry's `exclude`.

## Fixtures

0.2.0's eight, with the placed scenarios on 0.3.0; `normal-dependency-in-process`
(failing) and `dev-dependency-and-boundary-package` (passing).

A command's placed `schemas.rs` is a presentation boundary beside the CLI spec's output module: it encodes its own answer with `serde_json`, and the package that holds it may depend on `serde_json`.

The CLI spec's placed `src/output/schemas.rs` (template `schemas`), the rows every command prints the same way, is a presentation boundary beside the output module: it writes them with `serde_json`. Fixture `placed-cli-output-schemas` (passes).
