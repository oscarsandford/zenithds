pub mod error {
    use axum::{
        extract::Json,
        http::StatusCode,
        response::{Response, IntoResponse},
    };
    use serde::Serialize;

    pub enum ZenithError {
        FileSystemError(std::io::Error),
        RegexError(regex::Error),
        CSVError(csv::Error),
        PredicateError(String),
        QueryError(String),
        // more error types here as needed
    }

    fn server_error(error: ZenithError) -> (StatusCode, String) {
        eprintln!("Logging: {error}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong".to_owned()
        )
    }

    impl IntoResponse for ZenithError {
        fn into_response(self) -> Response {
            
            #[derive(Serialize)]
            struct ErrorResponse {
                message: String,
            }
    
            let (status, message) = match self {
                ZenithError::FileSystemError(error) => server_error(error.into()),
                ZenithError::RegexError(error) => server_error(error.into()),
                ZenithError::CSVError(error) => server_error(error.into()),
                ZenithError::PredicateError(error) => {
                    (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        format!("Incorrect predicate syntax: {error}")
                    )
                },
                ZenithError::QueryError(error) => {
                    (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        format!("Incorrect header, rows, or query body: {error}")
                    )
                },
                // Handle more errors here as needed
                // Client errors return more specific messages
            };
            
            (status, Json(ErrorResponse { message })).into_response()
        }
    }

    impl std::fmt::Display for ZenithError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ZenithError::FileSystemError(error) => write!(f, "File system IO error: {}", error),
                ZenithError::RegexError(error) => write!(f, "Regex error: {}", error),
                ZenithError::CSVError(error) => write!(f, "CSV read or write error: {}", error),
                ZenithError::PredicateError(error) => write!(f, "Predicate error: {}", error),
                ZenithError::QueryError(error) => write!(f, "Query error: {}", error),
            }
        }
    }

    impl From::<std::io::Error> for ZenithError {
        fn from(error: std::io::Error) -> Self { Self::FileSystemError(error) }
    }
    impl From::<regex::Error> for ZenithError {
        fn from(error: regex::Error) -> Self { Self::RegexError(error) }
    }
    impl From::<csv::Error> for ZenithError {
        fn from(error: csv::Error) -> Self { Self::CSVError(error) }
    }
}


pub mod query {
    use std::path::PathBuf;
    use serde::{Deserialize, Serialize};
    use regex::Regex;
    use super::error::ZenithError;

    /// Operations on a query predicate.
    #[derive(Deserialize, Debug)]
    pub enum PredOp {
        EQ,
        NE,
        LT,
        GT,
        LE,
        GE,
        CONTAINS,
    }

    // pub enum LogicalOperator {
    //     AND,
    //     OR,
    // }

    /// Used for evaluating values in rows.
    #[derive(Deserialize, Debug)]
    pub struct Predicate {
        pub field: String,
        op: PredOp,
        value: String,
        // No logical operators for now. Just assume
        // that multiple predicates are joined with AND.
        // logical_op: Option<LogicalOperator>
    }

    /// Metadata for a file in a collection.
    pub struct FileMetadata {
        pub filename: String,
        pub collection: String,
        pub filepath: PathBuf,
        pub size: u64,
    }

    /// A convenient way to group header and records. Can be removed later.
    #[derive(Deserialize, Serialize)]
    pub struct CSVData {
        pub header: Vec<String>,
        pub records: Vec<Vec<String>>,
    }

    impl Predicate {
        pub fn new(field: String, op: PredOp, value: String) -> Predicate {
            Predicate { field, op, value }
        }

        pub fn satisfied_by(&self, value: &String) -> bool {
            // Do we need to do some parsing to see if we can do int and
            // float comparisons? Or it is alright to leave them as strings?
            match self.op {
                PredOp::EQ => *value == self.value,
                PredOp::NE => *value != self.value,
                PredOp::LT => *value < self.value,
                PredOp::GT => *value > self.value,
                PredOp::LE => *value <= self.value,
                PredOp::GE => *value >= self.value,
                PredOp::CONTAINS => value.contains(&self.value),
            }
        }
    }

    /// A query description.
    pub struct DataQuery {
        pub fields: Vec<String>,
        pub predicates: Vec<Predicate>,
        pub filename_regex_predicates: Vec<Predicate>,
    }

    impl DataQuery {
        /// Create a new query. Directly sets the query `fields` with no changes.
        /// 
        /// Parses the list of `string_predicates` into two `Predicate` lists:
        /// - `predicates` contains predicates for rows
        /// - `filename_regex_predicates` contains regex predicates, to be run on the file names in the collection
        /// 
        /// The `predicates` are parsed from the form `field OP value`, where `OP` is a recognized operator.
        /// 
        /// The `filename_regex_predicates` are parsed from the form `HAS regex OP value`, where `regex` is a regular expression.
        /// 
        /// Raises a `PredicateError` if any of the strings
        /// in `string_predicates` cannot be converted into a `Predicate`.
        /// 
        pub fn new(
            fields: Vec<String>,
            string_predicates: Vec<String>
        ) -> Result<DataQuery, ZenithError> {
            // Parse predicates here. If there is a leading "HAS", the field is considered a regex.
            // Note that the value can be the empty string.
            let re = Regex::new(r"^(HAS |)(.+) (==|!=|<|>|<=|>=|CONTAINS) (.*)$")?;
            let mut predicates = Vec::new();
            let mut filename_regex_predicates = Vec::new();

            for s in string_predicates {
                // Considered to be a regex predicate if first group
                // is "HAS ", and as an ordinary predicate if it is the empty string.
                if let Some((_, [is_regex_field, field, op, value])) = re.captures(&s).map(|c| c.extract()) {
                    let pred_op = match op {
                        "==" => PredOp::EQ,
                        "!=" => PredOp::NE,
                        "<" => PredOp::LT,
                        ">" => PredOp::GT,
                        "<=" => PredOp::LE,
                        ">=" => PredOp::GE,
                        "CONTAINS" => PredOp::CONTAINS,
                        _ => return Err(ZenithError::PredicateError(format!("Incorrect predicate operator on {}", s)))
                    };
                    let p = Predicate::new(field.to_string(), pred_op, value.to_string());
                    if !is_regex_field.is_empty() {
                        filename_regex_predicates.push(p);
                    }
                    else {
                        predicates.push(p);
                    }
                }
                else {
                    return Err(ZenithError::PredicateError(format!("Incorrect format on predicate '{}'", s)));
                }
            }

            Ok(DataQuery { fields, predicates, filename_regex_predicates })
        }
    }
}


pub mod api {
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    pub struct CreatePayload {
        pub filename: String,
        pub header: Vec<String>,
        pub rows: Vec<Vec<String>>,
    }

    #[derive(Deserialize)]
    pub struct QueryParameters {
        pub page: Option<usize>,
        pub per_page: Option<usize>,
    }

    #[derive(Deserialize)]
    pub struct QueryPredicates {
        pub fields: Vec<String>,
        pub predicates: Vec<String>, // given as strings in api
    }

    #[derive(Serialize)]
    pub struct QueryResponse {
        pub header: Vec<String>,
        pub rows: Vec<Vec<String>>,
    }

    // api functions
}
