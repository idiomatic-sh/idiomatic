# no-persist-toggles-in-ports 0.1.0

The first release.

An identifier the entry lists in `toggles` (`Keeping`, `ImageKeeping`, `Writing`), written in a
port's interface or types file, is a finding: whether a cache is kept is the adapter's, injected
by its factory.

A settings value that the edge injects into a factory (`Store { directory, build, writing }`, in the domain) is not a port's type: it is read as written in a port's `interface.rs` and `types.rs` only, and a factory or an adapter that names `Writing` is not reported.
