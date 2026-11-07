# digest-not-stamp-alone 0.1.0

The first version. Text made from a modification time is a finding: a `{modified}` or `{mtime}` capture in a string
literal, or a `modified` or `mtime` value passed to a macro. Such text is a key that decides a file or a build is
unchanged from its stamp alone; the key holds the content's digest instead, and the stamp only decides whether to hash
again.

## Fixtures

`listing-keyed-by-stamp` and `build-keyed-by-stamp` (failing), `digest-gated-by-stamp` (passing).
