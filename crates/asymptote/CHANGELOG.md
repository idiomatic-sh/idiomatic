# Changelog

## [0.1.3] - 2026-11-16

### Features

- A `bmf` tool holds each of its measures to its best with a tolerance: `tolerance-percent` names each measure and the whole
  percent a run may rise over its best before it fails. A measure the table does not name is held to its best exactly.

### Fixes

- `up` honours the tolerance, and a tool's measures are judged and recorded with one best each.
