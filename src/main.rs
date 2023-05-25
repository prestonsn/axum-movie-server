use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    routing::post,
    Error, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio;
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Movie {
    name: String,
    slug: String,
    year: usize,
    desc: String,
}

unsafe impl Send for Movie {}
unsafe impl Sync for Movie {}

#[derive(Default)]
struct CommonState {
    db: HashMap<String, Json<Movie>>,
}

type SharedState = Arc<RwLock<CommonState>>;

#[axum_macros::debug_handler]
async fn movie_get(
    Path(slug): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<Movie>, StatusCode> {
    println!(" get() incoming json : {:?}", slug);
    match state.read().await.db.get(&slug) {
        Some(movie) => {
            return Ok(movie.clone());
        }
        None => return Err(StatusCode::NOT_FOUND),
    }
}

async fn movie_post(
    Json(payload): Json<Movie>,
    State(state): State<SharedState>,
) -> (StatusCode, String) {
    println!(" post() incoming json : {:?}", payload);
    // let slug = payload.slug;
    state
        .write()
        .await
        .db
        .insert(payload.slug.clone(), Json(payload.clone()));

    (StatusCode::OK, payload.slug)
}

#[tokio::main]
async fn main() {
    let shared_state = SharedState::default();

    let router = Router::new()
        .with_state(Arc::clone(&shared_state))
        .route("/movie/:slug", get(movie_get));
    // .route("/movie", get(movie_post)
    //     .post_service(movie_post(json, state)))
    // .with_state(Arc::clone(&shared_state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    axum::Server::bind(&"127.0.0.1:3000".parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
}
