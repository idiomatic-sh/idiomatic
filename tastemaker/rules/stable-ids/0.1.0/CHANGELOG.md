# stable-ids 0.1.0

The first version. A string literal without spaces that makes an id from an enumeration's counter is a finding: an id
prefix constant followed by a bare counter (`{RULE_PREFIX}{number}`), a `{position}` or `{ordinal}` capture, or a dash
followed by a counter at the end of the text (`rule-{n}`). The id comes from what the item is instead, so adding or
removing another item leaves it as it was.

## Fixtures

`rule-ids-by-number` and `rendered-files-by-position` (failing), `ids-from-definitions` (passing).
