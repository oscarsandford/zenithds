use axum::{
    body::Bytes,
    extract::{Json, Path, Query},
    routing::{get, post, delete},
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
        .route("/render", post(render_csv_v1))
        .route("/create/{collection}", post(create_csv_v1))
        .route("/delete/{collection}/{filename}", delete(delete_csv_v1))
        .route("/query/{collection}", post(query_post_v1));

    let app =  Router::new()
        .nest(config::prefix("v1").as_str(), api_routes_v1);

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


/// Renders a request `body` as CSV data, returning a `header` and `rows`.
async fn render_csv_v1(
    body: Bytes,
) -> Result<Json<QueryResponse>, ZenithError> {
    // Maybe we can put a check that the request header has set the
    // context type to CSV (e.g. error 415 unsupported media type).
    let (header, rows) = db::render(&body[..])?;
    Ok(Json( QueryResponse { header, rows } ))
}


/// Creates or overwrites a CSV as `filename` in
/// the `collection` with a given `header` and `rows`.
async fn create_csv_v1(
    Path(collection): Path<String>,
    Json(payload): Json<CreatePayload>,
) -> Result<(), ZenithError> {

    println!("Received a request to create '{}' in collection '{}', with a header of length {} and {} rows",
        payload.filename, collection, payload.header.len(), payload.rows.len());
    match db::insert(&collection, payload) {
        Ok(()) => {
            println!("Inserted in collection '{}'", collection);
            Ok(())
        },
        Err(err) => {
            eprintln!("The request to create in collection '{}' was unsuccessful", collection);
            Err(err)
        }
    }
}


/// Deletes a CSV as `filename` from the `collection`.
async fn delete_csv_v1(
    Path((collection, filename)): Path<(String, String)>,
) -> Result<(), ZenithError> {

    println!("Received a request to delete '{}' in collection '{}'", filename, collection);
    match db::delete(&collection, &filename) {
        Ok(()) => {
            println!("Deleted '{}' in collection '{}'", filename, collection);
            Ok(())
        },
        Err(err) => {
            eprintln!("The request to delete in collection '{}' was unsuccessful", collection);
            Err(err)
        }
    }
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
        .chunks(query.per_page.unwrap_or_else(|| config::envar_usize("ZENITHDS_DEFAULT_PAGE_SIZE")).max(1))
        .nth(query.page.unwrap_or_else(|| config::envar_usize("ZENITHDS_DEFAULT_PAGE")))
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
