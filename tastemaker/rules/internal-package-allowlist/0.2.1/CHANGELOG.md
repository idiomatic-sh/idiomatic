# internal-package-allowlist

## 0.2.1

Every join of the query now plans under the engine's growth check, which refuses a join keyed on the file alone unless one side is proven to hold each file once. A denied dependency takes its span from its manifest's document node, of which a manifest has one: the join is written `all-pairs join`, every pair being the answer. The findings and the scenarios are as in 0.2.0. It needs a Tastemaker whose RyoQL reads `all-pairs join`.

## 0.2.0

The query is unchanged; the version states and proves what it already matched. A dependency's identity is its `package =`, and its key when it names none, a local path dependency included. Scenarios `renamed-path-engine` (the 324 review's seed: `dom = { package = ... , path = ... }`) and `key-names-the-engine` (violations). The why says the rail holds only where placed, so a workspace places it on every member manifest.

## 0.1.0

- Check internal package identities across local, registry, git, workspace, target and dependency-kind spellings without treating binding names as package identities.
