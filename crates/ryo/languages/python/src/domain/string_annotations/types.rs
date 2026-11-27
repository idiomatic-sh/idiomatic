//! The types of the `string_annotations` concept: what a typing form is asked for, and the imports of a file that can name one.

use crate::domain::lexical_facts::types::QualifiedPath;
use crate::domain::model::lexical::types::TypedTree;
use crate::domain::model::view::types::{Built, SemanticModel};
use ryo::languages::Position;

/// What asks for the typing form of an expression: the forms it tells apart differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormUse {
    /// The object of a subscription: `Literal`, `Annotated` and the generic classes read their operands differently.
    Subscripted,
    /// The callee of a call: `cast`, `NewType` and `TypeVar` read some of their arguments as types.
    Called,
    /// The annotation of an annotated assignment: `TypeAlias` makes the value a type.
    Annotating,
}

/// The imports of a file that can name a typing form: the places of the aliases that bring a name of a typing module, and of
/// those that bring a name of another module with generic classes. Kept by the worker from file to file.
#[derive(Clone, Debug, Default)]
pub(crate) struct FormImports {
    /// The aliases of `typing` and `typing_extensions`.
    pub(crate) typing: Vec<Position>,
    /// The aliases of the other modules with generic classes.
    pub(crate) generic: Vec<Position>,
}

/// What a string's type context is decided through: the tree, the imports that can name a form, and the model, built the first
/// time a name has to be resolved.
#[derive(Debug)]
pub(crate) struct FormSource<'s, 't, 'a> {
    /// The tree.
    pub tree: TypedTree<'t, 'a>,
    /// The imports of the file that can name a typing form.
    pub(crate) imports: &'s FormImports,
    /// The model of the file.
    pub(crate) model: &'s mut SemanticModel,
    /// The parts of the model built for the tree in hand.
    pub(crate) built: &'s mut Built,
    /// The qualified name in hand.
    pub path: &'s mut QualifiedPath,
}

/// Whether a name can name a typing form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormNaming {
    /// An import of the file brings the name from a module with such forms, or it is a builtin generic class.
    Possible,
    /// Nothing brings it: it is no form, whatever it resolves to.
    Impossible,
}
