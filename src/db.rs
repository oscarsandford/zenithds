use std::{
    path::Path,
    collections::HashMap,
    sync::{mpsc, Arc},
    thread,
};
use regex::Regex;

use crate::types::{
    query::{CSVData, FileMetadata, Predicate, DataQuery},
    error::ZenithError,
    api::{QueryPredicates, CreatePayload},
};
use crate::config;


/// Read the CSV with `filename` from the `collection`,
/// returning its header and rows as determined by the `query`.
/// 
/// The header is automatically set on the first row found that is complete.
/// Rows before the header and rows with a different length than the header are ignored.
/// 
/// Make this function efficient.
fn read_csv(
    collection: &str,
    filename: &str,
    query: &Arc<DataQuery>,
) -> Result<CSVData, ZenithError> {

    let path = Path::new(config::DATA_PATH).join(collection).join(filename);
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(path)?;

    let mut records: Vec<Vec<String>> = Vec::new();
    let mut header: Vec<String> = Vec::new();

    for (_i, result) in reader.records().enumerate() {
        // Make this efficient (pass references instead of copying? use structs for specific structure?)
        // For now this will return an error if the result cannot be read.
        let record: Vec<String> = result?
            .into_iter()
            .map(|v| String::from_utf8(Vec::from(v)).unwrap_or_else(|_| String::from("")))
            .collect();

        // Append rows that match the length of the header.
        if !header.is_empty() && header.len() == record.len() {
            // Joining the header with the record allows easier lookups.
            let mut record_hashmap: HashMap<String, String> = HashMap::new();
            for (k, v) in header.iter().zip(record.iter()) {
                record_hashmap.insert(k.to_string(), v.to_string());
            }

            // If no predicates, we can go ahead.
            // Otherwise check if all predicates satisfied.
            // Predicates with a field not found in the header have no effect.
            if query.predicates.is_empty() || query.predicates.iter().all(|pred| {
                match record_hashmap.get(&pred.field) {
                    Some(v) => pred.satisfied_by(v), // field found
                    None => true, // field not found
                }
            }) {
                // If no fields specified, simply push the record.
                if query.fields.is_empty() && !record.is_empty() {
                    records.push(record);
                }
                // Otherwise filter the record values needed based on the fields specified.
                // This needs to be done here because we want to be able to apply
                // predicates on fields we might not necessarily want to return.
                else {
                    // This will order the record's values in the same order as the fields.
                    let filtered: Vec<String> = query.fields.iter()
                            .filter_map(|field| record_hashmap.get(field)) // get returns value in hashmap
                            .map(|s| s.to_owned())
                            .collect();
                    if !filtered.is_empty() {
                        records.push(filtered);
                    }
                }
            }
        }
        // Set the header automatically on the first record with complete fields.
        else if header.is_empty() && record.iter().all(|v: &String| v.len() > 0) {
            header = record;
        }
    }

    // Limit what is returned in the header.
    if !query.fields.is_empty() {
        header = query.fields.iter()
            .filter(|field| header.contains(field))
            .map(|field| field.to_owned())
            .collect();
    }

    Ok(CSVData { header, records })
}


/// Returns a list of files and their metadata in
/// the `collection`, filtered by any `filename_regex_predicates`.
fn list_collection_files(
    collection: &str,
    filename_regex_predicates: &Vec<Predicate>,
) -> Result<Vec<FileMetadata>, ZenithError> {

    // Compose each regex beforehand.
    let mut regex_predicates = Vec::new();
    for pr in filename_regex_predicates {
        match Regex::new(pr.field.as_str()) {
            Ok(re) => regex_predicates.push((re, pr)),
            Err(_) => return Err(ZenithError::PredicateError(format!("regex: '{}'", pr.field))),
        }
    }

    let path = Path::new(config::DATA_PATH).join(collection);
    let files_metadata: Vec<FileMetadata> = std::fs::read_dir(path)?
        .map(|entry| {
            match entry {
                Ok(e) => FileMetadata {
                    filename: e.file_name().into_string().unwrap_or_else(|_| String::from("")),
                    collection: String::from(collection),
                    filepath: e.path(),
                    size: match e.metadata() {
                        Ok(m) => m.len(),
                        Err(_) => 0,
                    }
                },
                Err(_) => FileMetadata {
                    filename: String::from(""),
                    collection: String::from(collection),
                    filepath: "".into(),
                    size: 0,
                }
            }
        })
        .filter(|m| {
            m.filename != "" && m.size > 0
            &&
            regex_predicates.iter().all(|(re, pr)| match re.find(&m.filename) {
                Some(ma) => pr.satisfied_by(&ma.as_str().to_string()),
                None => false,
            })
        })
        .collect();

    Ok(files_metadata)
}


/// Based on the `size` of each of the `files`, divide them into `k` groups.
/// 
/// Uses a round-robin approach for now. Sort the files by their
/// size, and then distribute them into each group one-by-one.
fn group_collection_files(
    mut files: Vec<FileMetadata>,
    k: usize
) -> Vec<Vec<FileMetadata>> {

    // Starting with a round-robin approach.
    // Sort the files by their size, and then distribute them
    // into each group one-by-one.
    files.sort_by_key(|m| m.size);

    let mut groups: Vec<Vec<FileMetadata>> = Vec::new();
    for _ in 0..k {
        groups.push(Vec::new());
    }

    for (i, file) in files.into_iter().rev().enumerate() {
        groups[i % k].push(file);
    }

    groups
}


/// Throws an error if the `header` is not the same as headers in the `collection`.
fn satisfies_collection_header(
    collection: &str,
    header: &Vec<String>,
)-> Result<(), ZenithError> {

    let collection_path = Path::new(config::DATA_PATH).join(collection);
    let entries: Vec<Result<std::fs::DirEntry, std::io::Error>> = std::fs::read_dir(&collection_path)?
        .take(3).collect();

    for e in entries {
        let entry = e?;
        let entry_path = collection_path.join(entry.file_name());
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(entry_path)?;
        let mut entry_header: Vec<String> = Vec::new();

        for result in reader.records() {
            let record: Vec<String> = result?
                .into_iter()
                .map(|v| String::from_utf8(Vec::from(v)).unwrap_or_else(|_| String::from("")))
                .collect();

            if record.iter().all(|v: &String| v.len() > 0) {
                entry_header = record;
                break;
            }
        }

        if entry_header.len() != header.len() ||
            entry_header.iter().zip(header).any(|(a, b)| a != b) {
            return Err(ZenithError::QueryError(format!(
                "Header {:?} does not match header in collection '{}'",
                header, &collection
            )));
        }
    }

    Ok(())
}


/// Make a selection on `collection` with `predicates`.
/// 
/// Returns the field names in a header as `Vec<String>` and rows of values as `Vec<Vec<String>>`.
/// 
/// Uses threads to divide the search computation. The header will be set
/// on the first header returned. Therefore, for now, we make the assumption
/// that all data in the collection has consistent headers. As the rows are
/// received in nondeterministic order, the order of the rows returned from
/// this function will vary. One can sort the rows to solve this.
pub fn select(
    collection: &str,
    predicates: QueryPredicates,
) -> Result<(Vec<String>, Vec<Vec<String>>), ZenithError> {

    let query = DataQuery::new(predicates.fields, predicates.predicates)?;
    let query = Arc::new(query); // drop this at end of function

    let files = list_collection_files(collection, &query.filename_regex_predicates)?;
    let groups = group_collection_files(files, config::envar_usize("ZENITHDS_NUM_WORKERS"));

    let (sender, receiver) = mpsc::channel();
    let mut threads = Vec::new();
    let (mut header, mut records): (Vec<String>, Vec<Vec<String>>) = (Vec::new(), Vec::new());

    let group_sizes: Vec<String> = groups.iter()
                        .map(|g| g.iter().map(|m| m.size).sum())
                        .map(|n: u64| format!("{}KB", n / 1000))
                        .collect();

    println!("SELECT '{}' with {} groups {:?}", &collection, groups.len(), group_sizes);

    for group in groups {
        let sender = sender.clone();
        let query = Arc::clone(&query);
        let join_handle = thread::spawn(move || {
            for fm in group {
                match read_csv(&fm.collection, &fm.filename, &query) {
                    Ok(data) => {
                        if let Err(err) = sender.send(data) {
                            eprintln!("read {}/{} send error: {}", &fm.collection, &fm.filename, err);
                        }
                    },
                    Err(err) => {
                        eprintln!("read {}/{} read error: {}", &fm.collection, &fm.filename, err);
                    }
                }
            }
        });
        threads.push(join_handle);
    }

    // Need to drop the initial sender here so the receiver will not be waiting for it.
    drop(sender);

    for mut received in receiver {
        if header.is_empty() {
            header = received.header;
        }
        records.append(&mut received.records);
    }

    for join_handle in threads {
        if let Err(err) = join_handle.join() {
            eprintln!("Failed to join thread: {:?}", err);
        }
    }

    drop(query);

    Ok((header, records))
}


/// Inserts `payload` into `collection`.
pub fn insert(
    collection: &str,
    payload: CreatePayload,
) -> Result<(), ZenithError> {

    if collection.is_empty() || payload.filename.is_empty() || payload.header.is_empty() {
        return Err(ZenithError::QueryError("Payload collection, filename, or header is empty".to_string()));
    }

    // Make sure the length of each given row matches the length of the given header.
    if payload.rows.iter().any(|row| row.len() != payload.header.len()) {
        return Err(ZenithError::QueryError(format!(
            "The length of a row does not match header of length {}", payload.header.len()
        )));
    }

    // Check the payload header to make sure it will work in this collection.
    satisfies_collection_header(collection, &payload.header)?;

    // Write the data to the collection.
    let insert_path = Path::new(config::DATA_PATH).join(collection).join(&payload.filename);
    let mut writer = csv::WriterBuilder::new().from_path(insert_path)?;

    writer.write_record(&payload.header)?;
    for row in payload.rows {
        writer.write_record(row)?;
    }

    Ok(())
}


/// Deletes `filename` from a `collection`, if it exists.
pub fn delete(
    collection: &str,
    filename: &str,
) -> Result<(), ZenithError> {

    if filename.is_empty() || collection.is_empty() {
        return Err(ZenithError::QueryError("The filename or collection is empty".to_string()));
    }
    let delete_path = Path::new(config::DATA_PATH).join(collection).join(filename);
    std::fs::remove_file(delete_path)?;
    Ok(())
}


/// Renders `bytes` as CSV data, returning the `header` and `rows`.
pub fn render(
    bytes: &[u8]
) -> Result<(Vec<String>, Vec<Vec<String>>), ZenithError> {

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(bytes);

    let mut records: Vec<Vec<String>> = Vec::new();
    let mut header: Vec<String> = Vec::new();

    for result in reader.records() {
        let record: Vec<String> = result?
            .into_iter()
            .map(|v| String::from_utf8(Vec::from(v)).unwrap_or_else(|_| String::from("")))
            .collect();

        // Append rows that match the length of the header.
        if !header.is_empty() && header.len() == record.len() {
            records.push(record);
        }
        // Set the header automatically on the first record with complete fields.
        else if header.is_empty() && record.iter().all(|v: &String| v.len() > 0) {
            header = record;
        }
    }

    Ok((header, records))
}
