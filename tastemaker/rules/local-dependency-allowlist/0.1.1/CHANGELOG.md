# local-dependency-allowlist 0.1.1

Every join of the query now plans under the engine's growth check, which refuses a join keyed on the file alone unless one side is proven to hold each file once. A denied dependency takes its span from its manifest's document node, of which a manifest has one: the join is written `all-pairs join`, every pair being the answer. The findings and the scenarios are as in 0.1.0. It needs a Tastemaker whose RyoQL reads `all-pairs join`.

# Changes

- Check resolved local manifest identities against declared sibling capabilities across every dependency kind; aliases and workspace/target declarations do not bypass the boundary.
