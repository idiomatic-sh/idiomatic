# layer-order

## 0.2.1

Every join of the query now plans under the engine's growth check, which refuses a join keyed on the file alone unless one side is proven to hold each file once. An upward edge takes its span from its manifest's document node, of which a manifest has one: the join is written `all-pairs join`, every pair being the answer. The findings and the scenarios are as in 0.2.0. It needs a Tastemaker whose RyoQL reads `all-pairs join`.

## 0.2.0

A package directly under the workspace root (`cli/Cargo.toml`) has a layer as a nested one does: 0.1.0 captured a manifest only beneath a parent directory, so a top-level package was never ranked and its upward edges went unreported (the command line's `edges-apart` placement over the SDK counted 0). Scenario `top-level-edge-depends-on-the-other-edge` (violation); the 0.1.0 scenarios are kept.

## 0.1.0

A local non-dev path dependency never runs from a package in a lower layer to one in a higher layer.
