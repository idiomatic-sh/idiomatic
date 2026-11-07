# fixture-fakes 0.1.0

The widened `fixture-fakes` extends the `hex` spec's owned
`fixture-fakes` from "fake only the hex's own ports" to "fake exactly the hex's own ports",
so a bench or a test can replace any port of a command's graph with its fake.

A finding is, as before, an `impl <Name>Interface for` in a hex's `fixtures.rs`,
`fixtures/<fixture>.rs` or `fixtures/<fixture>/fixture.rs` whose trait the hex does not
declare; and, new, a port (a trait named `<Name>Interface`) declared in the hex's
`interface.rs` or in one of its `use_cases/<use_case>/interface.rs` that none of those files
implements, found at the trait. Another trait in a port file owes no fake. A plugin hex declares no port, so it owes no fake. Several fakes of one port (a
working one and a failing one) stay allowed.

Staged here, unplaced, because the `hex` spec's working version is the one the products
follow: replacing its owned rule now would turn every port without a fake into a finding
before the products have one. A later change moves this directory over
`specs/hex/0.10.0/rules/fixture-fakes/`, renames the fixtures' rule to
`idiomatic.arch/hex/fixture-fakes`, writes the missing fakes, and deletes this directory.
