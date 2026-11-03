//! project_missing_a_manifest_relation: the compiler rejects this.

use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_language::domain::answers::types::{Support};
use ryo_language::inbound::use_cases::project::errors::ProjectError;
use ryo_language::inbound::use_cases::project::interface::ProjectUseCase;
use ryo_language::inbound::use_cases::project::types::{ManifestName, ManifestReader, ProjectRequest};

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

    fn resolve(&self, _request: &ProjectRequest) -> Result<ryo_language::inbound::use_cases::project::types::Resolved, ProjectError> {
        Ok(ryo_language::inbound::use_cases::project::types::Resolved { refused: Vec::new(), unlisted: Vec::new(), consulted: Vec::new(), derived: () })
    }

    ryo_relations::each_relation!(all_but_package_dependency);
}

fn main() {}
