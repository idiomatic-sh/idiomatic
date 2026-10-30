//! The `language` service: free functions named by verb over the domain's types, with no IO and
//! no ports. It says which files are typescript, and in which dialect each is read.

use super::constants::{
    COMMONJS_EXTENSION, DECLARATIONS_END, DECLARATIONS_MARK, EXTENSIONS, JSX_EXTENSION,
    MODULE_EXTENSION, PLAIN_EXTENSION,
};
use crate::domain::packages::constants::SEPARATOR;
use crate::domain::syntax::types::Dialect;
use ryo::languages;

/// Whether `path` names a typescript file: one ending in the language's extension.
#[must_use]
pub(crate) fn reads(path: languages::SourcePath<'_>) -> languages::Recognition {
    if EXTENSIONS
        .iter()
        .any(|extension| path.0.ends_with(extension))
    {
        languages::Recognition::Reads
    } else {
        languages::Recognition::Declines
    }
}

/// The dialect the file at `path` is read in, as TypeScript reads its extension: `.tsx` with JSX, `.mts` always a module, `.cts` always
/// `CommonJS`, and a file whose name marks it as declarations (`index.d.ts`, `index.d.mts`, `styles.d.css.ts`) as declarations of its
/// kind; any other file is plain TypeScript.
#[must_use]
pub(crate) fn dialect(path: languages::SourcePath<'_>) -> Dialect {
    let name = path.0.rsplit(SEPARATOR).next().unwrap_or(path.0);
    if name.ends_with(JSX_EXTENSION) {
        return Dialect::Tsx;
    }
    // `.d.mts` and `.d.cts` name declarations when the mark ends the name before the extension and something comes before it.
    let marked_before = |extension| {
        name.strip_suffix(extension).is_some_and(|base| {
            base.len() > DECLARATIONS_END.len() && base.ends_with(DECLARATIONS_END)
        })
    };
    if name.ends_with(MODULE_EXTENSION) {
        return if marked_before(MODULE_EXTENSION) {
            Dialect::ModuleDeclarations
        } else {
            Dialect::Module
        };
    }
    if name.ends_with(COMMONJS_EXTENSION) {
        return if marked_before(COMMONJS_EXTENSION) {
            Dialect::CommonJsDeclarations
        } else {
            Dialect::CommonJs
        };
    }
    // `.ts` names declarations when the mark is anywhere in the name but at its start.
    if name.ends_with(PLAIN_EXTENSION) && name.rfind(DECLARATIONS_MARK).is_some_and(|at| at != 0) {
        Dialect::Declarations
    } else {
        Dialect::TypeScript
    }
}
