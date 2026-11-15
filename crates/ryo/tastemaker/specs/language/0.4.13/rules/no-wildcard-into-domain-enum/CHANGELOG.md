# no-wildcard-into-domain-enum 0.1.0

The first version: a `_ =>` arm in a `match` of an adapter.rs or a schemas.rs is a finding, so that a variant added to a domain enum is a compile error in every adapter that maps it. An approximation: the syntax cannot tell a match that builds a domain enum from any other, so every match of the two files is read.
