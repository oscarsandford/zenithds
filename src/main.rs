use axum::{
    extract::{Json, Path, Query},
    routing::{get, post},
    Router,
};
use std::time::Instant;

pub mod types;
pub mod config;
pub mod db;

use crate::types::{
    error::ZenithError,
    api::*,
};


#[tokio::main]
async fn main() {
    let api_routes_v1 = Router::new()
        .route("/", get(root))
        .route("/upload", post(upload_csv_v1))
        .route("/create/{collection}", post(create_csv_v1))
        .route("/query/{collection}", get(query_get_v1))
        .route("/query/{collection}", post(query_post_v1));

    let app =  Router::new()
        .nest("/api/v1", api_routes_v1);

    if let Ok(listener) = tokio::net::TcpListener::bind(config::address()).await {
        println!("ZenithDS: Establish listener on {}", config::address());
        if let Err(_) = axum::serve(listener, app).await {
            println!("Could not create server on {}. Exiting.", config::address());
        }
    }
    else {
        println!("Could not establish server on {}. Exiting.", config::address());
    }
}

async fn root() -> &'static str {
    "Welcome to ZenithDS"
}


async fn upload_csv_v1(
    Json(_payload): Json<UploadPayload>,
) -> Result<(), ZenithError> {
    Ok(())
}

async fn create_csv_v1(
    Json(_payload): Json<CreatePayload>,
) -> Result<(), ZenithError> {
    Ok(())
}


/// Queries a `collection`, returning the result of
/// a query using only query parameters as predicates.
async fn query_get_v1(
    Path(collection): Path<String>,
    Query(query): Query<QueryParameters>
) -> Result<Json<QueryResponse>, ZenithError> {

    let mut predicates = Vec::new();
    if let Some(ref date_start) = query.date_start {
        predicates.push(format!("__date_start >= {}", date_start.to_string()));
    }
    if let Some(ref date_end) = query.date_end {
        predicates.push(format!("__date_end <= {}", date_end.to_string()));
    }

    query_post_v1(
        Path(collection),
        Query(query),
        Json(QueryPredicates { fields: vec![], predicates: predicates })
    ).await
}


/// Queries a `collection` based on `predicates`,
/// returning a `header` and `rows`.
async fn query_post_v1(
    Path(collection): Path<String>,
    Query(query): Query<QueryParameters>,
    Json(predicates): Json<QueryPredicates>,
) -> Result<Json<QueryResponse>, ZenithError> {

    let now = Instant::now();
    let (header, rows) = db::select(&collection, predicates)?;

    match rows
        .chunks(query.per_page.unwrap_or_else(|| config::envar_usize("DEFAULT_PAGE_SIZE")).max(1))
        .nth(query.page.unwrap_or_else(|| config::envar_usize("DEFAULT_PAGE")))
    {
        Some(paged_rows) => {
            println!("Returned {} fields and {}/{} rows in {:.2?}", header.len(), paged_rows.len(), rows.len(), now.elapsed());
            Ok(Json( QueryResponse { header, rows: paged_rows.to_owned() } ))
        },
        None => {
            println!("No rows in {:.2?}", now.elapsed());
            Ok(Json( QueryResponse { header, rows: vec![] } ))
        }
    }
}
