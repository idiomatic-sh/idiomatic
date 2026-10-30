# kind-suffix

## 0.1.0

The first release. It replaces the `Interface` suffix of `interface-suffix` for the crates that hold parts: the part spec places it on a use case's and a port's `interface.rs` with the part's kind, and the namespace specs place it on a collection's with `kind = "collection"`. The hex spec keeps `interface-suffix`.

a port the SDK re-exports in extension_points.rs is declared under its plain noun, so this rule no longer demands the `Port` suffix of a port; it refuses the retired `Interface` suffix and another kind's suffix, and `extension-point-ports` holds the `Port` suffix to the ports that are not exposed.

- Param `exposed`; an outbound port's trait ends in `Port` unless the port is listed, and a listed port's trait does not end in another kind's suffix.

- A handle, a `pub trait` that a method of another trait in the file returns, is not the interface: it is neither counted nor named by the kind, matching the amended `interface-shape`. Without this the two rules contradicted each other. Fixtures: `port-with-handle`, `unreturned-second-trait`.
