//! project_missing_a_manifest_relation: the compiler rejects this.

use ryo_language::domain::answers::types::Support;
use ryo_language::inbound::use_cases::project::errors::ProjectError;
use ryo_language::inbound::use_cases::project::interface::ProjectUseCase;
use ryo_language::domain::files::types::ManifestName;
use ryo_language::inbound::use_cases::project::types::{ManifestReader, ProjectRequest};
use ryo_relations::domain::answers::types::AbsenceCause;

/// Answers every manifest relation of the catalog but `package_dependency`.
macro_rules! all_but_package_dependency {
    (
        syntax { $( $syntax:tt )* }
        manifest { $( $variant:ident $module:ident $row:ident $column:ident ; )* }
        engine { $( $engine:tt )* }
    ) => {
        $( all_but_package_dependency!(@one $module $variant); )*
    };
    (@one package_dependency PackageDependency) => {};
    (@one $module:ident $variant:ident) => {
        fn $module(
            &self,
        ) -> Support<ManifestReader<ryo_relations::domain::relations::collections::relations::$module::item::$variant>> {
            ryo_language::domain::answers::service::absent(AbsenceCause::NoManifests)
        }
    };
}

struct Missing;

impl ProjectUseCase for Missing {
    type Derived = ();

    fn manifests(&self) -> &'static [ManifestName] {
        &[]
    }

    fn primary_role(
        &self,
        _path: ryo_language::domain::files::types::SourcePath<'_>,
    ) -> ryo_language::domain::files::types::PrimaryFileRole {
        ryo_language::domain::files::service::primary_role_from_manifests(
            _path,
            self.manifests().iter().copied(),
        )
    }

    fn project_entries(
        &self,
        _request: &ProjectRequest,
        _entries: &mut ryo_language::domain::files::types::ProjectEntries,
    ) {
    }

    fn resolve(
        &self,
        _request: &ProjectRequest,
    ) -> Result<ryo_language::inbound::use_cases::project::types::Resolved, ProjectError> {
        Ok(ryo_language::inbound::use_cases::project::types::Resolved {
            observed_entries: Vec::new(),
            refused: Vec::new(),
            unlisted: Vec::new(),
            consulted: Vec::new(),
            derived: (),
        })
    }

    ryo_relations::each_relation!(all_but_package_dependency);
}

fn main() {}
