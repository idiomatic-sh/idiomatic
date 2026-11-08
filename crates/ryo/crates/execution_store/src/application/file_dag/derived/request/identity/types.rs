//! A request compared by its exact static fields.
use ryo_execution_domain::domain::derived_views::types::ViewRequest;

/// Two of these are equal when their requests match field for field, as request preparation requires.
pub(in crate::application::file_dag::derived) struct StaticRequest<'a>(pub &'a ViewRequest);
