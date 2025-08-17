use axum::{
    routing::get,
    Router,
    extract::Query,
    Json,
};
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use std::sync::Arc;

use helios::SearchEngine;

#[derive(Debug, Deserialize)]
struct SearchParams {
    q: String,
}

#[tokio::main]
async fn main() {
    
    let engine = Arc::new(SearchEngine::new("./")); // later: configurable path

    let app = Router::new()
        .route("/search", get(search_handler))
        .with_state(engine);

    // Run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running at http://{}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn search_handler(
    state: axum::extract::State<Arc<SearchEngine>>,
    Query(params): Query<SearchParams>,
) -> Json<Vec<String>> {
    let results = state.search(&params.q);
    Json(results)
}
