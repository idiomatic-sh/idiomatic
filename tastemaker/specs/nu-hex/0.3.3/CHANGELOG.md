# nu-hex 0.3.3

The condition that frees a crate from the inbound surface is `library-only`, answered by the spec's `library-only` rule (was `domain-only`): a library root that declares its domain or its application public and no inbound module. A crate a product splits its engine into, an application layer with no use case of its own (ryo's execution store), is placed like the values crate: no `src/inbound.rs`. Fixtures `root-pub-mod-application-answers-yes` and `application-with-inbound`. Nothing else changes.

# nu-hex 0.3.2

Places `use-case@0.1.2`, where a use case splits its test support along concept into `fixtures/<concept>.rs` with no contract. Nothing else changes.

# nu-hex 0.3.1

Places the corrected rules (`public-fields@0.2.0`, `constants-once@0.2.0`, `mirrored-types@0.2.0`, `no-json-in-process@0.4.0`) and the specs that place them at their corrected versions, where 0.3.0 named the earlier ones. Nothing else changes.
