# layer-order

## 0.2.0

A package directly under the workspace root (`cli/Cargo.toml`) has a layer as a nested one does: 0.1.0 captured a manifest only beneath a parent directory, so a top-level package was never ranked and its upward edges went unreported (the command line's `edges-apart` placement over the SDK counted 0). Scenario `top-level-edge-depends-on-the-other-edge` (violation); the 0.1.0 scenarios are kept.

## 0.1.0

A local non-dev path dependency never runs from a package in a lower layer to one in a higher layer.
