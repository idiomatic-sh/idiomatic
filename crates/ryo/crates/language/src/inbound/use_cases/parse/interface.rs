//! What the `parse` use case promises: the one contract a language implements to be read by ryo. A
//! language implements this and `project`, and those two are all of ryo it implements: it says which
//! files it reads, builds their trees, and answers, for every syntax relation of the catalog, how it reads it or
//! that it is not in the language. The method of each relation is generated from the catalog's one list
//! (`domain/relations/catalog.rs`) and has no default body, so a relation added to the catalog stops every
//! language from compiling until it has answered.

use super::errors::ParseError;
use super::types::{KindNames, ParseConfigurationIdentity, ParseRequest, SkippedDirectory};
use crate::domain::answers::types::{Reader, Support};
use crate::domain::elements::types::{Completion, TableBuilder, Tree};
use crate::domain::relations::types::Count;
use crate::domain::terms::types::UnknownName;
use crate::inbound::use_cases::parse::types::Recognition;

/// What the methods of a trait are made of, one for each syntax relation of the catalog.
macro_rules! syntax_methods {
    (
        syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        manifest { $( $manifest:tt )* }
        engine { $( $engine:tt )* }
    ) => {
        $(
            #[doc = stringify!($module)]
            fn $module(
                &self,
            ) -> Support<Reader<$crate::domain::relations::collections::relations::$module::item::$variant, Self::FileModel>>;
        )*
    };
}

/// What a language offers for its files.
pub trait ParseUseCase: Send + Sync {
    /// Optional semantic capabilities this provider promises to answer, including proven language absence.
    fn capabilities(&self) -> &'static [crate::domain::relations::types::OptionalCapability];

    /// Canonical immutable parser settings that can change the tree or diagnostics.
    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_>;

    /// What the language works out of a file once, when it parses it, for its readers to read: an index of the file, so that no reader
    /// searches the file again for each element it is given. A language that works out nothing has `()`. The engine makes one for each
    /// worker, keeps it between files and gives it back to `file_model` to refill, so that a file costs no allocation the model of the last
    /// did not already pay.
    type FileModel: Default + Send + 'static;

    /// Refills `model`, which holds what the last file left in it, with the model of the file whose tree is `tree`: it clears what it
    /// keeps and writes the file's, and the readers of the file read it.
    fn file_model(&self, tree: &Tree<'_>, model: &mut Self::FileModel);

    /// Whether the language reads the file the request names, by its path.
    fn recognizes(&self, request: &ParseRequest<'_>) -> Recognition;

    /// The names of the directories the language never reads, at any depth: the dependency, environment and build directories of its
    /// ecosystem (`node_modules`, `target`), which both ryo's walk of a repository and Tastemaker's do not enter. A language whose
    /// ecosystem has none answers an empty list, and says in its constants why.
    fn skips(&self) -> &'static [SkippedDirectory];

    /// How many kinds of node and token the language's trees have, in its own numbering. It is at most the
    /// number of kinds a walk can tell apart.
    fn kind_count(&self) -> Count;

    /// The names of the kinds the language's trees have, each once: the words a query can write for a kind of node or token. A name
    /// the language's trees have no kind of is a word no query can match, which the engine refuses when a query writes it.
    fn kind_names(&self) -> KindNames;

    /// The reasons the language declares for a value of a column it cannot resolve statically (a wildcard import, a call of a variable):
    /// a closed list, each named once and shown by a fixture row. A reader that meets such a construct writes its row with the
    /// unknown value of the reason, and never omits the row or marks the whole file partial.
    fn unknown_reasons(&self) -> &'static [UnknownName];

    /// Reads the file the request names with the parser library, into the tree `builder` builds: opens a node, adds a token,
    /// closes a node. It is all the language does with the library, and the engine composes the rest around it: `build` makes
    /// the table with it, and the allocation row reads a file through a builder that builds nothing, so that what the library
    /// alone costs is this method and no other code.
    ///
    /// # Errors
    ///
    /// `Library` when the parser library fails. Bytes that are not valid text are never refused: the valid text is read, and the rest is
    /// an error token with a diagnostic (the contract, "A file is read, never refused").
    fn read_tree(
        &self,
        request: &ParseRequest<'_>,
        builder: &mut TableBuilder<'_>,
    ) -> Result<(), ParseError>;

    /// Finishes the table `read_tree` built for the file: names the kinds its elements have (`name_kind`), and writes the notes the
    /// request asks for, bytes only the language reads (`write_notes`). It reaches nothing else of the table.
    fn complete(&self, request: &ParseRequest<'_>, completion: &mut Completion<'_>);

    crate::each_relation!(syntax_methods);
}
