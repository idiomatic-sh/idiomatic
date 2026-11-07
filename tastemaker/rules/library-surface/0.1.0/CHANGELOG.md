# library-surface 0.1.0

A library is named only by the paths of its surface: the entry lists the library's crate in `library` and every path the product may name in `surface`, so a product's promise to call a few functions of another product's SDK is enforced, not asserted. Fixtures `off-the-surface` (violation) and `on-the-surface` (pass, with a crate whose name starts like the library's).
