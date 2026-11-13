# nu-hex 0.3.5

Places its ports at `port@0.3.3`, which places its adapters at `adapter@0.4.0`. Everything else is as in 0.3.4.

# nu-hex 0.3.4

Places its parts at the versions that place nested parts and nested groups by their roles: `application-service@0.1.2`, `use-case@0.1.3` and `concept@0.1.2` (`port@0.3.1` as before), so an application operation's or a domain concept's nested part, and a use case's nested group of cases, need no line of a product's manifest. The include patterns are as in 0.3.3. Everything else is as in 0.3.3. 0.3.3 is removed with this version, not kept as published: no lock names it, nothing has been released, and an unreleased version keeps no copy for compatibility.

Edited in place: places `no-runtime-fixtures@0.2.0`, `io-at-boundary@0.2.0`, where it named their earlier versions, so a product that places it locks one version of each rule: two versions of one rule share its id, and the earlier entries ran the later query without their params (a `test-shape@0.1.0` entry reported nothing).

Edited in place: an application operation's module file, `src/application/<application>.rs`, is placed by `part-module@0.1.0` (alias `application-module`) where it was placed by `application-module@0.1.0`, and held by `test-group-namespace@0.1.0` with `case-table = "refused"` (entry `application-namespace`), as a nested part's module file is: it declares the operation's modules and holds none of their items. An operation's service and its test stay required by `application-service@0.1.2`, and `module-declares` still requires each to be declared, so a module that holds an operation keeps both; a namespace operation, which only gathers sub-concepts (`application-service@0.1.2`'s `namespace` condition), declares its sub-concepts and nothing else (owner via PM, 2026-11-12: ryo's execution store's `file_dag`). Scenarios `namespace-application-module` (passes), `namespace-application-module-holding-a-fn` (refused at the function) and `application-without-service-or-sub-concepts` (its service and test reported missing). The one requirement of `application-module@0.1.0` that `part-module` does not carry, a public service, is kept by the spec's new rule `public-application-service`: a module file beside an operation directory holding `service.rs` declares `pub mod service;` (`mod service;` and `pub(crate) mod service;` are reported; a namespace operation, with no service.rs, is not read). A placement cannot pick its template file by file, so the two kinds of module file share `part-module` and the rule tells them apart. Scenario `operation-module-with-a-private-service` (refused), and the rule's `public-service-passes`, `private-service-refused`, `crate-visible-service-refused`, `namespace-is-not-read`.

Edited in place: `public-application-service` also keeps the other line `application-module@0.1.0` wrote, the proof's declaration: a module file beside an operation directory holding `service.rs` and `service_test.rs` declares `mod service_test` with no visibility, gated by exactly `#[cfg(test)]` and decorated by nothing else (`module-declares` only asks that the gate name `test`, so `#[cfg(any(test, feature = "fixtures"))]` passed it; leak review of ryo's claude/carveout-v, 2026-11-12). The entry also reads `src/application/*/service_test.rs`. Scenario `operation-module-with-a-fixtures-gated-test` (refused), and the rule's `exact-test-gate-passes`, `fixtures-gated-test-refused`, `visible-test-refused`.

# nu-hex 0.3.3

The condition that frees a crate from the inbound surface is `library-only`, answered by the spec's `library-only` rule (was `domain-only`): a library root that declares its domain or its application public and no inbound module. A crate a product splits its engine into, an application layer with no use case of its own (ryo's execution store), is placed like the values crate: no `src/inbound.rs`. Fixtures `root-pub-mod-application-answers-yes` and `application-with-inbound`. Nothing else changes.

# nu-hex 0.3.2

Places `use-case@0.1.2`, where a use case splits its test support along concept into `fixtures/<concept>.rs` with no contract. Nothing else changes.

# nu-hex 0.3.1

Places the corrected rules (`public-fields@0.2.0`, `constants-once@0.2.0`, `mirrored-types@0.2.0`, `no-json-in-process@0.4.0`) and the specs that place them at their corrected versions, where 0.3.0 named the earlier ones. Nothing else changes.
