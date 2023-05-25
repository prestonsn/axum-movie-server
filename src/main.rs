use axum::{
    body::Bytes,
    error_handling::HandleErrorLayer,
    extract::{DefaultBodyLimit, Path, State},
    handler::Handler,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    routing::{delete, get},
    Error, Json, Router,
};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Movie {
    name: String,
    slug: String,
    year: usize,
    desc: String,
}

// unsafe impl Send for Movie {}
// unsafe impl Sync for Movie {}

#[derive(Default)]
struct CommonState {
    db: HashMap<String, Json<Movie>>,
}

type SharedState = Arc<RwLock<CommonState>>;

async fn movie_post(State(state): State<SharedState>, Json(payload): Json<Movie>) -> StatusCode {
    println!(" post() incoming json : {:?}", payload);
    // let slug = payload.slug;
    state
        .write()
        .unwrap()
        .db
        .insert(payload.slug.clone(), Json(payload.clone()));

    StatusCode::OK
}

#[tokio::main]
async fn main() {
    let shared_state = SharedState::default();

    let router = Router::new()
        .route(
            "/:slug",
            get(movie_get).with_state(Arc::clone(&shared_state)),
        )
        .route("/", post(movie_post).with_state(Arc::clone(&shared_state)));

    // let post_router = Router::new().route("/", post(movie_post).with_state(shared_state));
    let app = Router::new().nest("/movies", router);

    axum::Server::bind(&"127.0.0.1:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn movie_get(
    Path(slug): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<Movie>, StatusCode> {
    println!(" get() incoming json : {:?}", slug);
    match state.read().unwrap().db.get(&slug) {
        Some(movie) => {
            return Ok(movie.clone());
        }
        None => return Err(StatusCode::NOT_FOUND),
    }
}
