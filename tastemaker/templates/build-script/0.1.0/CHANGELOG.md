# build-script

## 0.1.0

The first release: a crate's `build.rs`, which declares `main`. Tastemaker's authoring crate
embeds the identity of the source it is built from with one.

The content is the command line's build script in full, the one that writes `src/languages.rs`, so a
scaffolded build script is the working one and not a bare `main`.

Each language is handed to the language factory as `LanguageHandles::from(language)`, `language` being `ryo_<name>::linked()`: the SDK's `Language` has no method that gives its engine, so the script names none.
