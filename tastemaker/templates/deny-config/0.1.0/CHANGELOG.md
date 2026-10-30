# deny-config

## 0.1.0

`[bans] deny` names the crates that would break clig.dev's interactivity and signal
guidelines: `dialoguer`, `inquire` and `rpassword` (a command line never prompts) and
`ctrlc` and `signal-hook` (Ctrl-C ends it at once). New failing fixture
`prompt-crate-allowed`.

The first v2 release: a product's cargo-deny policy: licenses, bans and sources, checked by the cargo-deny gate.
