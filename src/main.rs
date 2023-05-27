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

    let pool = state.read().await.db.clone();
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

    match res {
        Ok(res) => {
            state
                .write()
                .await
                .cache
                .insert(new_movie.id, Json(new_movie));
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

    match state.read().await.cache.get(&req_id) {
        Some(resp) => {
            tracing::debug!(" Cache hit, serving cached result {:?}", req_id);
            return Ok(resp.clone());
        }

        None => {
            tracing::debug!("Cache miss, accessing db...");
            let mut pool = state.read().await.db.get_owned().await.unwrap();
            let res: Json<Movie> = Json(
                schema::movies::table
                    .find(req_id)
                    .first(&mut pool)
                    .await
                    .map_err(internal_error)
                    .unwrap(),
            );

            tracing::debug!("Got requested movie, updating cache.");
            state.write().await.cache.insert(req_id, res.clone());

            return Ok(res);
        }
    };
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "axum-moviesdb=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = std::env::var("DATABASE_URL").unwrap();
    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_url);
    let pool = bb8::Pool::builder().build(config).await.unwrap();

    let shared_state = RwLock::new(CommonState {
        cache: HashMap::new(),
        db: pool,
    });

    let router = Router::new()
        .route("/:req_id", get(get_movie))
        .route("/", post(create_movie))
        .with_state(Arc::new(shared_state));

    let app = Router::new().nest("/movies", router);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
