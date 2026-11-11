# nu-hex 0.3.4

Places its parts at the versions that place nested parts and nested groups by their roles: `application-service@0.1.2`, `use-case@0.1.3` and `concept@0.1.2` (`port@0.3.1` as before), so an application operation's or a domain concept's nested part, and a use case's nested group of cases, need no line of a product's manifest. The include patterns are as in 0.3.3. Everything else is as in 0.3.3. 0.3.3 is removed with this version, not kept as published: no lock names it, nothing has been released, and an unreleased version keeps no copy for compatibility.

Edited in place: places `no-runtime-fixtures@0.2.0`, `io-at-boundary@0.2.0`, where it named their earlier versions, so a product that places it locks one version of each rule: two versions of one rule share its id, and the earlier entries ran the later query without their params (a `test-shape@0.1.0` entry reported nothing).

# nu-hex 0.3.3

The condition that frees a crate from the inbound surface is `library-only`, answered by the spec's `library-only` rule (was `domain-only`): a library root that declares its domain or its application public and no inbound module. A crate a product splits its engine into, an application layer with no use case of its own (ryo's execution store), is placed like the values crate: no `src/inbound.rs`. Fixtures `root-pub-mod-application-answers-yes` and `application-with-inbound`. Nothing else changes.

# nu-hex 0.3.2

Places `use-case@0.1.2`, where a use case splits its test support along concept into `fixtures/<concept>.rs` with no contract. Nothing else changes.

# nu-hex 0.3.1

Places the corrected rules (`public-fields@0.2.0`, `constants-once@0.2.0`, `mirrored-types@0.2.0`, `no-json-in-process@0.4.0`) and the specs that place them at their corrected versions, where 0.3.0 named the earlier ones. Nothing else changes.
