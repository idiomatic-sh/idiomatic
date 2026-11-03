//! The `project` service: free functions that drive an implementation of the contract. They read the
//! manifests through the language, then give each wanted manifest relation to the reader the language
//! answers for it, which writes the rows of its relation and no other.

use super::errors::ProjectError;
use super::interface::ProjectUseCase;
use super::types::{
    ManifestInput, ManifestItem, ManifestName, ManifestReader, ProjectEngine, ProjectRequest,
    Resolved,
};
use crate::domain::answers::types::{Answer, Support};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{Relation, RowWriter};
use crate::domain::symbols::types::Symbols;

/// What the reading of the manifests is made of, generated from the catalog's one list.
macro_rules! manifest_readings {
    (
        syntax { $( $syntax_variant:ident $syntax_module:ident $syntax_row:ident $syntax_column:ident ; )* }
        manifest { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        engine { $( $engine_variant:ident $engine_module:ident $engine_row:ident $engine_column:ident ; )* }
    ) => {
        /// Writes the rows of the wanted manifest relations the language reads, in the order the catalog lists them.
        fn write_wanted<P>(
            language: &P,
            input: &ManifestInput<'_, P::Derived>,
            symbols: &mut Symbols,
            out: &mut dyn RowWriter,
        ) where P: ProjectUseCase + ?Sized {
            $(
                if input.request.wanted.contains(&Relation::$variant) {
                    if let Answer::Reads(reader) = language.$module().as_ref() {
                        (reader.read)(input, symbols, out);
                    }
                }
            )*
        }

        /// Why the language does not read `relation` from the manifests, or nothing when it does or when the
        /// relation is not one of the manifests'.
        #[must_use]
        pub fn absence<P>(language: &P, relation: Relation) -> Option<AbsenceCause> where P: ProjectUseCase + ?Sized {
            match relation {
                $( Relation::$variant => match language.$module().as_ref() {
                    Answer::Reads(_) => None,
                    Answer::NotInThisLanguage(why) => Some(*why),
                }, )*
                $( Relation::$syntax_variant => None, )*
                $( Relation::$engine_variant => None, )*
            }
        }
    };
}

ryo_relations::each_relation!(manifest_readings);

/// Reads the manifests of the project and writes the rows of the wanted manifest relations into `out`.
/// Answers the bytes the run keeps for the next one, the manifests the language refused and the files it read beyond them.
///
/// # Errors
///
/// What the language's `resolve` answers.
pub fn project<P>(
    language: &P,
    request: &ProjectRequest,
    symbols: &mut Symbols,
    out: &mut dyn RowWriter,
) -> Result<Resolved, ProjectError>
where
    P: ProjectUseCase + ?Sized,
{
    let resolved = language.resolve(request)?;
    let input = ManifestInput {
        request,
        derived: &resolved.derived,
    };
    write_wanted(language, &input, symbols, out);
    Ok(Resolved {
        refused: resolved.refused,
        unlisted: resolved.unlisted,
        consulted: resolved.consulted,
        derived: (),
    })
}

/// The answer of a language that reads a relation from the manifests of a project through the item `I`.
#[must_use]
pub fn reads_manifests<I>() -> Support<ManifestReader<I::Relation, I::Derived>>
where
    I: ManifestItem,
{
    Support::from(Answer::Reads(I::READER))
}

/// For `impl ProjectUseCase`: answers every manifest relation as not in the language, for the test doubles of this crate only.
macro_rules! absent_manifests {
    (
        syntax { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        manifest { $( $manifest_variant:ident $manifest_module:ident $manifest_row:ident $manifest_column:ident ; )* }
        engine { $( $engine_variant:ident $engine_module:ident $engine_row:ident $engine_column:ident ; )* }
    ) => {
        $(
            fn $manifest_module(
                &self,
            ) -> $crate::domain::answers::types::Support<
                $crate::inbound::use_cases::project::types::ManifestReader<
                    ::ryo_relations::domain::relations::collections::relations::$manifest_module::item::$manifest_variant,
                >,
            > {
                $crate::domain::answers::service::absent(::ryo_relations::domain::answers::types::AbsenceCause::NoManifests)
            }
        )*
    };
}

/// Every `ProjectUseCase` is a `ProjectEngine`: the engine reads it through the contract's own drive.
impl<P: ProjectUseCase> ProjectEngine for P {
    fn manifest_names(&self) -> &'static [ManifestName] {
        ProjectUseCase::manifests(self)
    }

    fn project(
        &self,
        request: &ProjectRequest,
        symbols: &mut Symbols,
        out: &mut dyn RowWriter,
    ) -> Result<Resolved, ProjectError> {
        project(self, request, symbols, out)
    }

    fn absence_of(&self, relation: Relation) -> Option<AbsenceCause> {
        absence(self, relation)
    }
}
