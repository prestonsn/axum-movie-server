mod schema;

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, Path, State},
    http::{request::Parts, StatusCode},
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

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};
use tokio;

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

#[derive(Default)]
struct CommonState {
    cache: HashMap<String, Json<Movie>>,
}

struct DatabaseConnection(
    bb8::PooledConnection<'static, AsyncDieselConnectionManager<AsyncPgConnection>>,
);

fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[async_trait]
impl<S> FromRequestParts<S> for DatabaseConnection
where
    S: Send + Sync,
    Pool: FromRef<S>,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = Pool::from_ref(state);

        let conn = pool.get_owned().await.map_err(internal_error)?;

        Ok(Self(conn))
    }
}

// async fn movie_post(State(state): State<SharedState>, Json(payload): Json<Movie>) -> StatusCode {
//     println!(" post() incoming json : {:?}", payload);
//     // let slug = payload.slug;
//     state
//         .write()
//         .unwrap()
//         .cache
//         .insert(payload.slug.clone(), Json(payload.clone()));

//     StatusCode::OK
// }

async fn create_movie(
    Json(new_movie): Json<Movie>,
    State(pool): State<Pool>,
) -> Result<Json<Movie>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = diesel::insert_into(schema::movies::table)
        .values(new_movie)
        .returning(Movie::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;
    Ok(Json(res))
}

async fn get_movie(
    Path(req_id): Path<i32>,
    DatabaseConnection(mut conn): DatabaseConnection,
) -> Result<Json<Movie>, StatusCode> {
    println!(" get() incoming id : {}", req_id);
    let res = schema::movies::table
        .find(req_id)
        .first(&mut conn)
        .await
        .map_err(internal_error)
        .unwrap();

    Ok(Json(res))
}

// async fn movie_get(
//     Path(slug): Path<String>,
//     State(state): State<SharedState>,
// ) -> Result<Json<Movie>, StatusCode> {
//     println!(" get() incoming json : {:?}", slug);
//     match state.read().unwrap().cache.get(&slug) {
//         Some(movie) => {
//             return Ok(movie.clone());
//         }
//         None => return Err(StatusCode::NOT_FOUND),
//     }
// }

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "axum-moviesdb=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let shared_state = SharedState::default();

    let db_url = std::env::var("DATABASE_URL").unwrap();
    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_url);
    let pool = bb8::Pool::builder().build(config).await.unwrap();

    let router = Router::new()
        .route("/", post(create_movie))
        .route("/:req_id", get(get_movie))
        .with_state(pool);
    // .route(
    //     "/:slug",
    //     get(movie_get).with_state(Arc::clone(&shared_state)),
    // )
    // .route("/", post(movie_post).with_state(Arc::clone(&shared_state)));

    let app = Router::new().nest("/movies", router);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
