use axum::routing::get;
use axum::Router;
use todo_api::routes::todos::routes::{count, list};

// TODO: read the address from the environment (planted: Banned patterns)
const ADDRESS: &str = "0.0.0.0:3000";

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/todos", get(list))
        .route("/todos/count", get(|| async { count() }));
    let listener = tokio::net::TcpListener::bind(ADDRESS).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
