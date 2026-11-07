# JavaScript corpus

## Source

`acorn-cli.js` is the unchanged `bin/acorn` entrypoint distributed in Acorn's npm package. It exercises a hashbang, directive and CommonJS call.

`component.jsx`, `jsx-in-js.js` and `comparison.js` are written by this provider: a React component in `.jsx`, JSX in a `.js` file (an attribute named `class`, a `this.` member tag), and a `.js` file whose `<` is only ever a comparison. Their reference trees come from Acorn with acorn-jsx 5.3.2 (MIT).

## Version

Acorn 8.15.0, MIT license in `NOTICE-LICENSE`; acorn-jsx 5.3.2, MIT.
