/// A todo as the store keeps it, and as the routes answer it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Todo {
    pub id: u64,
    pub title: String,
    pub done: bool,
}
