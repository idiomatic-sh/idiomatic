# no-inline-modules

## 0.1.0

A role file declares no inline module with a body; a module holding only traits (the sealed-trait idiom) is the exception. Placed by idiomatic-rust over the role files.

A module holding only `use` items and macro invocations is the second exception: the scope an item-writing macro expands into (ryo's `kinds!` test expands it twice, each in its own module).
