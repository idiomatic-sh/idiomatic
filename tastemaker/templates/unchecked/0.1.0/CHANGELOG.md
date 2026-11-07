# unchecked

## 0.1.0

The first version: a file placed as it is, its contents unchecked. The repository root's `tastemaker.toml` is closed over the whole repository, and lists each top-level directory and file it holds with an entry of its own; a directory whose contents are their owner's (`tools/`, the dotfile directories) is placed by `file = "<directory>/{*path}"`, and a file by its name. A top-level file or folder no entry lists is a `#closed` finding.
