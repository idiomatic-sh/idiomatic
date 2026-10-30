# port-takes-no-settings 0.1.0

The first release.

A port method that takes or gives a setting is a finding: a type the entry lists in `settings`
(build identities, worker counts, memory budgets, per-call constants) anywhere in a parameter or a
result, a machine path parameter named as a location (`directory`, `cache`, `store`, `temp`,
`scratch`), and a machine path a method of such a name hands back. A path a request names (the
repository's root, a package's directory) and a path the port finds for the caller are not.

The types are read as written, token by token, so `&super::types::BuildName` and
`Result<WorkerCount, E>` count. Not seen: a setting under an alias of another name.

A port that hands the core a project of its own, with a `root` the port's other calls take as the repository, is not reported: the adapter owns where the project lives and removes it when the handle is dropped.
