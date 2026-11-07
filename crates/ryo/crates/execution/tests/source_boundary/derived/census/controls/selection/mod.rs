//! Controls of the selection and preparation entries of the derived owner.
use super::{Acquire, refuses};

pub(in super::super) static SELECTOR_PLAN: Acquire = Acquire {
    positive: r#"
mod census_selector_plan {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    use crate::domain::{derived_views::types::contract::value::DerivedContract, operating::errors::OperatingError, stored_selection::types::Selector};
    use std::sync::Arc;
    fn plan(owner: &DerivedOwner, program: &Arc<Selector>) -> Result<Option<Arc<DerivedContract>>, OperatingError> { owner.selector_plan(program) }
    const _: () = { let _ = plan; };
}
"#,
    negative: refuses(
        r#"
mod census_selector_plan_denied {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    fn peek(owner: &DerivedOwner) { let _ = &owner.backend; }
    const _: () = { let _ = peek; };
}
"#,
        "E0616",
        "backend",
        "private",
    ),
};
pub(in super::super) static SELECT: Acquire = Acquire {
    positive: r#"
mod census_select {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    const _: () = { let _ = DerivedOwner::select; };
}
"#,
    negative: refuses(
        r#"
mod census_select_denied {
    const _: () = { let _ = crate::application::file_dag::derived::owner::selection::select; };
}
"#,
        "E0603",
        "select",
        "private",
    ),
};
pub(in super::super) static PREPARE_PROGRAM_FILE: Acquire = Acquire {
    positive: r#"
mod census_prepare_program_file {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    const _: () = { let _ = DerivedOwner::prepare_program_file; };
}
"#,
    negative: refuses(
        r#"
mod census_prepare_program_file_denied {
    use crate::application::file_dag::derived::owner::DerivedOwner;
    fn peek(owner: &DerivedOwner) { let _ = &owner.file_prepared; }
    const _: () = { let _ = peek; };
}
"#,
        "E0616",
        "file_prepared",
        "private",
    ),
};
pub(in super::super) static PREPARE_VIEWS: Acquire = Acquire {
    positive: r#"
mod census_prepare_views {
    use crate::application::file_dag::derived::owner::selection::preparation::service::prepare_views_with_schemas;
    use crate::domain::{operating::errors::OperatingError, stored_selection::types::Selector, derived_views::types::contract::value::PreparedSelectionViews};
    use ryo_relations::domain::relations::types::{CellKind, Relation};
    use std::sync::Arc;
    fn prepare<'s>(selectors: Vec<Arc<Selector>>, resolver: &dyn Fn(Relation) -> &'s [CellKind], port: Arc<dyn crate::outbound::derived::interface::DerivedPort>) -> Result<Arc<PreparedSelectionViews>, OperatingError> { prepare_views_with_schemas(selectors, resolver, port) }
    const _: () = { let _ = prepare; };
}
"#,
    negative: refuses(
        r#"
mod census_prepare_views_denied {
    const _: () = { let _ = crate::application::file_dag::derived::owner::selection::preparation::service::activate_program; };
}
"#,
        "E0603",
        "activate_program",
        "private",
    ),
};
