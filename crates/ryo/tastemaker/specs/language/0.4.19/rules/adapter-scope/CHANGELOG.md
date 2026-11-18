
Domain concepts are no longer out of an adapter's reach: an adapter may import the domain of its own crate and of a kernel crate. It still reaches no other hex's adapters, no use case, and one port only.

The use-case arm is gone from adapter-scope: an adapter of one crate may import another crate's use-case interface, types and errors (a translation adapter), and import-directions forbids an adapter its own crate's use cases.
