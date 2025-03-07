use std::env;

fn unpack_var_usize(v: &str, default: usize) -> usize {
    env::var(v).unwrap_or_else(|_| default.to_string()).parse().unwrap_or(default)
}
fn unpack_var_str(v: & str, default: &str) -> String {
    env::var(v).unwrap_or_else(|_| default.to_string()).to_string()
}

pub const DATA_PATH: &'static str = if cfg!(debug_assertions) { "./data" } else { "/data" };
pub const DEFAULT_COLLECTION: &'static str = "main";

const NUM_WORKERS: usize = 4;
const DEFAULT_PAGE: usize = 0;
const DEFAULT_PAGE_SIZE: usize = 10;
const HOST: &str = "0.0.0.0";
const PORT: usize = 8750;

/// Retrieve the value of environment variable `v` as a `usize`.
/// 
/// Returns `0` if variable name not found, or the default if not set.
pub fn envar_usize(v: &str) -> usize {
    match v {
        "ZENITHDS_NUM_WORKERS" => unpack_var_usize(v, NUM_WORKERS),
        "ZENITHDS_DEFAULT_PAGE" => unpack_var_usize(v, DEFAULT_PAGE),
        "ZENITHDS_DEFAULT_PAGE_SIZE" => unpack_var_usize(v, DEFAULT_PAGE_SIZE),
        "ZENITHDS_PORT" => unpack_var_usize(v, PORT),
        _ => 0,
    }
}

/// Retrieve the value of environment variable `v` as a `String`.
/// 
/// Returns the empty string if variable name not found, or the default if not set.
pub fn envar_str(v: &str) -> String {
    match v {
        "ZENITHDS_HOST" => unpack_var_str(v, HOST),
        "ZENITHDS_USE_PREFIX" => unpack_var_str(v, ""),
        "ZENITHDS_ALLOWED_ORIGINS" => unpack_var_str(v, ""),
        _ => "".to_string(),
    }
}

/// Get the address for establishing the data service server.
/// 
/// Uses the values set in `HOST` and `PORT`.
/// In debug mode, the host name is `127.0.0.1`.
pub fn address() -> String {
    if cfg!(debug_assertions) {
        format!("127.0.0.1:{}", envar_usize("ZENITHDS_PORT"))
    }
    else {
        format!("{}:{}", envar_str("ZENITHDS_HOST"), envar_usize("ZENITHDS_PORT"))
    }
}

/// Returns the API resource prefix for the given `version`.
/// If `ZENITHDS_USE_PREFIX` is set, prepends the application name.
pub fn prefix(version: &str) -> String {
    if envar_str("ZENITHDS_USE_PREFIX").is_empty() {
        format!("/api/{version}")
    }
    else {
        format!("/zenithds/api/{version}")
    }
}
