mod schema;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    routing::post,
    Json, Router,
};

use tracing;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use diesel::prelude::*;
use diesel_async::{
    pooled_connection::AsyncDieselConnectionManager, AsyncPgConnection, RunQueryDsl,
};

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Insertable, Selectable, Queryable)]
#[diesel(table_name = schema::movies)]
struct Movie {
    id: i32,
    title: String,
    year: i32,
    description: String,
}

type SharedState = Arc<RwLock<CommonState>>;
type Pool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

struct CommonState {
    cache: HashMap<i32, Json<Movie>>,
    db: Pool,
}

fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[axum::debug_handler]
async fn create_movie(
    State(state): State<SharedState>,
    Json(new_movie): Json<Movie>,
) -> StatusCode {
    tracing::debug!(" create_movie() request {:?}", new_movie);

    let reader_lock = state.read().await;
    let pool = (*reader_lock).db.clone();
    let mut conn = match pool.get().await.map_err(internal_error) {
        Ok(conn) => conn,
        Err(e) => {
            let status = e.0;
            tracing::error!("Failed to acquire Diesel connection to db");
            return status;
        }
    };

    let res = diesel::insert_into(schema::movies::table)
        .values(new_movie.clone())
        .returning(Movie::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error);

    drop(reader_lock);
    match res {
        Ok(res) => {
            let mut writer_lock = state.write().await;
            (*writer_lock).cache.insert(new_movie.id, Json(new_movie));
            tracing::debug!("\t added movie to db, updated cache. {:?}", res)
        }
        Err(_e) => return StatusCode::ALREADY_REPORTED,
    }

    StatusCode::OK
}

#[axum::debug_handler]
async fn get_movie(
    State(state): State<SharedState>,
    Path(req_id): Path<i32>,
) -> Result<Json<Movie>, StatusCode> {
    tracing::debug!(" get_movie() id {:?}", req_id);
    println!(" get_movie() id {:?}", req_id);

    let reader_lock = state.read().await;
    let movie_json = match (*reader_lock).cache.get(&req_id) {
        Some(resp) => {
            tracing::debug!(" Cache hit, serving cached result {:?}", req_id);
            println!(" cache_hit ");
            resp.clone()
        }

        None => {
            tracing::debug!("Cache miss, accessing db...");
            println!(" cache miss ");
            let mut pool = (*reader_lock).db.get_owned().await.unwrap();
            let res: Json<Movie> = Json(
                schema::movies::table
                    .find(req_id)
                    .first(&mut pool)
                    .await
                    .map_err(internal_error)
                    .unwrap(),
            );

            res
        }
    };
    drop(reader_lock);
    tracing::debug!("Got requested movie, updating cache.");
    let mut writer_lock = state.write().await;
    (*writer_lock).cache.insert(req_id, movie_json.clone());

    Ok(movie_json)
}

#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "axum-moviesdb=debug,tower_http=debug")
    }
    tracing_subscriber::fmt::init();

    // tracing_subscriber::registry()
    //     .with(
    //         tracing_subscriber::EnvFilter::try_from_default_env()
    //             .unwrap_or_else(|_| "axum-moviesdb=debug,tower_http=debug".into()),
    //     )
    //     .with(tracing_subscriber::fmt::layer())
    //     .init();

    let db_url = std::env::var("DATABASE_URL").unwrap();
    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_url);
    let pool = bb8::Pool::builder().build(config).await.unwrap();

    let shared_state = RwLock::new(CommonState {
        cache: HashMap::new(),
        db: pool,
    });

    let router = Router::new()
        .route("/:req_id", get(get_movie).layer(TraceLayer::new_for_http()))
        .route("/", post(create_movie).layer(TraceLayer::new_for_http()))
        .with_state(Arc::new(shared_state));

    let app = Router::new().nest("/movies", router);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
