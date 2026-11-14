# allocation-test

## 0.3.0

The template takes variables, so any crate places its allocation test from it, not only the kernel language crate: `doc`,
the module doc; `owner` and `case`, the case's name; `fixture`, `double` and `field`, the fixture module, its double and the
double's boolean that puts it within or past the bound; `holds` and `breaks`, the two assertions' messages. Rendered with
the language crate's values it is 0.2.0's text byte for byte. It stays `match = "text"`. 0.2.0 is deleted: no lock in the
repository reads it once the ryo workspace moves to 0.3.0 (its release tags keep it).

## 0.2.0

The template is placed closed: `match = "text"`, where 0.1.0 had the default `contains`. The content is the
kernel language crate's allocation test as it now is: the module doc, the `#[global_allocator]` static and the `main` that
runs the one case on libtest-mimic. The language double it judges, its readers and the function that runs the row moved
into the parse use case's fixtures (`fixtures::allocating`), so the test names no internal of the relations crate and has
no `use` item. The `extra-content` fixture, which passed under 0.1.0, is now a finding; `missing-allocator` is one
finding. 0.1.0 is deleted.
