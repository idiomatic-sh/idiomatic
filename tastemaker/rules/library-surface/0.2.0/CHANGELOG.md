# library-surface

## 0.2.0

An `extern crate` is read as an import of the crate it names, so an alias (`extern crate X as Y`) is resolved to the library where it is declared and reported there when the library is not on the surface; a `use X as Y`, at the crate root or elsewhere, already names the library and is reported as before. Scenarios `an-extern-crate-alias` (the 324 review's seed), `a-root-use-alias` (violations) and `an-extern-crate-of-another-crate` (pass). A Cargo-renamed dependency stays with `internal-package-allowlist`.

## 0.1.0

A library is named only by the paths of its surface: the entry lists the library's crate in `library` and every path the product may name in `surface`, so a product's promise to call a few functions of another product's SDK is enforced, not asserted. Fixtures `off-the-surface` (violation) and `on-the-surface` (pass, with a crate whose name starts like the library's).
