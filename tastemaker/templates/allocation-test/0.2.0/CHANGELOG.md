# allocation-test

## 0.2.0

The template is placed closed: `match = "text"`, where 0.1.0 had the default `contains`. The content is the
kernel language crate's allocation test as it now is: the module doc, the `#[global_allocator]` static and the `main` that
runs the one case on libtest-mimic. The language double it judges, its readers and the function that runs the row moved
into the parse use case's fixtures (`fixtures::allocating`), so the test names no internal of the relations crate and has
no `use` item. The `extra-content` fixture, which passed under 0.1.0, is now a finding; `missing-allocator` is one
finding. 0.1.0 is deleted.
