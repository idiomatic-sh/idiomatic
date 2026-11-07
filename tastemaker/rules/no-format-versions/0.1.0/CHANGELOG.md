# no-format-versions 0.1.0

The first release. A format version is a promise to read old bytes; a store that does not
decode is rebuilt empty instead, and a file a person writes is read as it is today. The rule
reports the versions Asymptote's cache and lock, ryo's image and Tastemaker's manifest and
source inventory carried.

## What it reads

- A `const` or `static` named `MAGIC`, `VERSION` or `FORMAT`, or ending `_MAGIC`,
 `_VERSION` or `_FORMAT`, whose value is an integer or byte string literal.
- A named field `version`, `format`, or ending `_version` or `_format`, of a primitive
 integer type.
- A struct, enum or type alias named `FormatVersion` or ending in it.
- A string literal with `-v` or `_v` before a digit (`manifest-v4.postcard`).

## Allowed

- A release version: `env!("CARGO_PKG_VERSION")`, a string `"0.1.0"`, a `version: String`.

## Not seen

- A version written under another name, or computed.

A committed lock's version, declared as `LOCK_VERSION` (or ending in it), is not reported: a lock carries `version = 1` until the first release.

The `version` field every operation of a lock directory states is the lock's documented version: a lock adapter's `schemas.rs` (`outbound/lock/adapters/<format>/`) may read it, and its constant is `LOCK_VERSION`. The same field in any other adapter, and a constant called `VERSION` in a lock adapter, are reported.
