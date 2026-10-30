# idiomatic-sh-asymptote-core

asymptote's core: the use cases its command line calls, the rules that judge a run, and its ports.

## Domains

### captures

The `{name}`s a tool's inputs write, what each takes on disk, and the tool each match makes.

### git

The words of git's command line the adapters that run git share.

### hooks

The git hook scripts `init` writes, and how it knows a script is its own.

### inputs

The Merkle tree over a check's input files, and the digest each tool's pathspecs come to.

### judgement

What a run's findings mean against the accepted records: new, resolved or unjudged.

### manifest

The tools a product's `asymptote.toml` names.

### operations

The immutable lock operations and their fold.

### perf

What a tool measured, judged against the best the lock records for it.

### ratchet

What `up` records and what `down` drains.

### renames

Carrying accepted records through git's renames.

### reports

What a check leaves for `up`, `down` and the next check.

### seeding

The perf records `up` writes from the last check's medians.

### selection

Which new findings `up` records.

### violations

What a tool reported, and the key that identifies it.
