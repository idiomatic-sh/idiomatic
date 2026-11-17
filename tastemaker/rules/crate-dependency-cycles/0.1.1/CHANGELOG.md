# crate-dependency-cycles 0.1.1

Adds the scenario `three-cycles-and-a-self-dependency`: three cycles of local packages (through a dev-dependency, through a
build dependency, and of three packages) are reported at every manifest on them; a package that dev-depends on itself for its
fixtures feature, and a package that only depends into a cycle, are not. The query is as in 0.1.0.

# crate-dependency-cycles 0.1.0

The first release: a local package on a cycle of path dependencies of the kinds `kinds` names is reported at its manifest.
