# extension-point-ports 0.1.0

A port whose trait the SDK re-exports in `extension_points.rs` has no `adapters.rs`, `factory.rs` or `adapters/` in the core, and every other port has at least one adapter. The port spec's counts for those three are zero so this rule decides.

a port the SDK exposes is declared under its plain noun (`Language`, not `LanguagePort`) and every other port keeps the `Port` suffix; the rule holds both, with `kind-suffix` no longer demanding the suffix of a port.

the SDK has no extension_points.rs; a port is exposed when an SDK resource module (`sdk/src/resources/<resource>.rs`) re-exports its trait.

- Param `exposed`; the list must equal the ports the SDK resource modules re-export (a re-export not listed and a listed port not re-exported are findings).

Only the port's own trait must end in `Port` when the port is not exposed: a handle trait its methods return (a `Reader`, a `Worker`) keeps its plain noun.
