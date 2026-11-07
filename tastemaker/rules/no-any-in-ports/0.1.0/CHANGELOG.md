# no-any-in-ports 0.1.0

The first release.

The identifier `Any` and a method named `into_any`, written in a port's interface file, are
findings: a caller that can downcast reaches past the port to the adapter's own type.

Read in the port's `types.rs` too, not only its `interface.rs`: a handle that wraps `Any` is as much a downcast. Fixture `an-any-held-in-a-port-type` (finding).
