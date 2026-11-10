# idiomatic-project

## 0.3.2

`no-tracker-text-shipped` also reports a tracker id with no digit and no child number when it stands in parentheses (`(ryo-abcd)`, `(ryo-abcd census)`) or after "bead" (`bead ryo-abcd`, `bead_ryo-abcd`, `bead-ryo-abcd`): agents write ids that way, and a bare digitless id stays out because it reads like a package name (`idiomatic-sh-ryo-sdk`). Everything else is as in 0.3.1.

## 0.3.1

`crate-dependency-depth` allows a package to reach fourteen local packages, not nine, and only the ryo workspace places this version: ryo's decomposition of its engine into a stack of crates (the domain, the store and the runtime below execution) adds internal packages behind the sdk, and every language provider reaches the packages the sdk reaches plus the sdk. Everything else is as in 0.3.0, which stays as published.
