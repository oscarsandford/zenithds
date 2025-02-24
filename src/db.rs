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
    api::QueryPredicates,
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
/// the `collection`, filtered by any `filename_predicates`.
fn list_collection_files(
    collection: &str,
    filename_predicates: &Vec<Predicate>,
) -> Result<Vec<FileMetadata>, ZenithError> {

    // This regex can be moved to a separate configuration, or make regex predicates.
    let re = Regex::new(r"(_\d{8})|(_\d{4}_\d{2}_\d{2})")?;

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
            // Filename filtering helps lower search space
            match re.find(&m.filename) { // remove underscores
                Some(ma) => filename_predicates.iter().all(|p| p.satisfied_by(&ma.as_str().to_string())),
                None => true
            }
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

    let files = list_collection_files(collection, &query.filename_predicates)?;
    let groups = group_collection_files(files, config::envar_usize("NUM_WORKERS"));

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
