# language-depends-on-ryo-only

## 0.1.0

The first version. Placed over `languages/*/Cargo.toml` by ryo's tastemaker.toml.

Edited in place: the SDK's package is `idiomatic-sh-ryo-sdk` (the command line took `idiomatic-sh-ryo`), which the `idiomatic-sh-ryo-*` glob would otherwise find as an engine crate; a package of that name is the SDK and is not a finding.
