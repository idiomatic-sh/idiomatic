# gitignore

## 0.1.0

The first v2 release: the repository's `.gitignore`, ignoring `target/` (Cargo's build output), `.bin/` (where cargo-run-bin installs each workspace's pinned tools) and `.tastemaker/` (Tastemaker's analysis cache) at any depth. A `.gitignore` is placed, not parsed: the check requires the file, and generation writes the rendered template.

The file lists `.ryo_cache/` too: the directory `ryo` keeps what a run came to in, at the root of each project it lints. A product's `.gitignore` names it, and the corpus never lists it.

It lists `.tastemaker_cache/` too: the directory Tastemaker keeps its store in, with ryo's own store inside it, at the root of each project it checks.
