# mirrored-types 0.2.0

A struct that declares a lifetime parameter is not compared: it is the borrowed view of data another struct owns (a record read in
place from encoded bytes beside the owned value it is converted to once), a second shape by ownership and not a copy of the concept.
Two owned structs of the same fields are reported as before.
