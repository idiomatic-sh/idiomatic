# distribution-tools 0.2.0

The registries and the memory bench are matched at the product root, where `tm check` reads them (`distribution/npm/package.json`, `distribution/pypi/pyproject.toml`, `cli/benches/memory/main.rs`, `asymptote.toml`). 0.1.0 captured a `{*product}/` directory in front of each path, which only its scenarios' `demo/` files had: on a product it matched nothing, so a missing `memory` tool or a smoke tool without `--test npm` passed. A product is one workspace, so each file is held to the tools of its one `asymptote.toml`. The scenarios' files sit at the root. Everything else is as in 0.1.0.

## Moving up from 0.1.0

- Nothing to change in a product wired as agreed; one that is not now gets the findings 0.1.0 missed.
