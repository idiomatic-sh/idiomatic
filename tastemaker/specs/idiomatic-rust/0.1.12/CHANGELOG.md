# idiomatic-rust 0.1.12

`directions-domain` forbids `*::inbound::use_cases::*::interface::*UseCase`: a domain names no crate's use-case trait, so a domain service never takes `&impl` or `&dyn` of another crate's workflow. What it needs is a port the domain declares, which its parent implements over the use case and hands down. The pattern names the trait, not the `interface` module: an import's target is the item it names, so `*::interface` alone matched no import of the trait and fired nothing. The types and errors an interface module re-exports stay open. Everything else is as in 0.1.11.
