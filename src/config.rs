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
// const DEFAULT_HEADER_LINE: usize = 11;
const HOST: &str = "0.0.0.0";
const PORT: usize = 8750;

/// Retrieve the value of environment variable `v` as a `usize`.
/// 
/// Returns `0` if variable name not found, or the default if not set.
pub fn envar_usize(v: &str) -> usize {
    match v {
        "NUM_WORKERS" => unpack_var_usize(v, NUM_WORKERS),
        "DEFAULT_PAGE" => unpack_var_usize(v, DEFAULT_PAGE),
        "DEFAULT_PAGE_SIZE" => unpack_var_usize(v, DEFAULT_PAGE_SIZE),
        // "DEFAULT_HEADER_LINE" => unpack_var_usize(v, DEFAULT_HEADER_LINE),
        "PORT" => unpack_var_usize(v, PORT),
        _ => 0,
    }
}

/// Retrieve the value of environment variable `v` as a `String`.
/// 
/// Returns the empty string if variable name not found, or the default if not set.
pub fn envar_str(v: &str) -> String {
    match v {
        "HOST" => unpack_var_str(v, HOST),
        _ => "".to_string(),
    }
}

/// Get the address for establishing the data service server.
/// 
/// Uses the values set in `HOST` and `PORT`.
/// In debug mode, the host name is `127.0.0.1`.
pub fn address() -> String {
    if cfg!(debug_assertions) {
        format!("127.0.0.1:{}", envar_usize("PORT"))
    }
    else {
        format!("{}:{}", envar_str("HOST"), envar_usize("PORT"))
    }
}
