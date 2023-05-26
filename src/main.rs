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

struct CommonState {
    cache: HashMap<i32, Json<Movie>>,
    db: Pool,
}

unsafe impl Send for CommonState {}

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

#[axum::debug_handler]
async fn create_movie(
    State(state): State<SharedState>,
    Json(new_movie): Json<Movie>,
) -> StatusCode {
    println!(" post() incoming new movie {:?}", new_movie);
    let pool = state.read().unwrap().db.clone();
    let mut conn = match pool.get().await.map_err(internal_error) {
        Ok(conn) => conn,
        Err(e) => return e.0,
    };

    let res = diesel::insert_into(schema::movies::table)
        .values(new_movie)
        .returning(Movie::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error);
    match res {
        Ok(_res) => println!("Wrote to DB!"),
        Err(_e) => return StatusCode::INTERNAL_SERVER_ERROR,
    }
    StatusCode::OK
}

#[axum::debug_handler]
async fn get_movie(
    // DatabaseConnection(mut conn): DatabaseConnection,
    State(state): State<SharedState>,
    Path(req_id): Path<i32>,
) -> Result<Json<Movie>, StatusCode> {
    println!(" get() incoming id : {}", req_id);

    let cached_response = match state.read().unwrap().cache.get(&req_id) {
        Some(resp) => {
            println!("Using cached entry!");
            return Ok(resp.clone());
        }

        None => {}
    };

    let mut pool = state.read().unwrap().db.get_owned().await.unwrap();
    let res: Json<Movie> = Json(
        schema::movies::table
            .find(req_id)
            .first(&mut pool)
            .await
            .map_err(internal_error)
            .unwrap(),
    );

    state.write().unwrap().cache.insert(req_id, res.clone());

    Ok(res)
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

    let db_url = std::env::var("DATABASE_URL").unwrap();
    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_url);
    let pool = bb8::Pool::builder().build(config).await.unwrap();

    let shared_state = RwLock::new(CommonState {
        cache: HashMap::new(),
        db: pool,
    });

    let router = Router::new()
        // .route("/:req_id", get(get_movie))
        .route("/", post(create_movie))
        .with_state(Arc::new(shared_state));
    // .with_state(shared_state);
    // .with_state(pool);
    // .with_state(shared_state)

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
