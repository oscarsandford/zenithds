# zenithds

ZenithDS is a data service using CSV for data storage. This software is designed to be configured and extended based on domain use cases.

## Endpoints

The data service currently supports a REST API. Some of the names may change.

Here, unless otherwise noted, a `header` is a list of fields/columns as strings, and `rows` is a list of lists with values as strings.

### POST `/api/{version}/query/{collection}`
  
Queries the data in a `collection`. Takes `predicates` that can influence the rows returned, and `fields` which influence the fields/columns returned. Returns a `header` and `rows`.

The `fields` are simply strings matching the names of the fields. The `predicates` are strings, and can be in one of two forms:

- Row-level predicates: `field OP value`, where `OP` can be any operator recognized by the program
- File name predicates: `HAS regex OP value`, where `regex` is a regular expression

Row-level predicates limit which rows are returned by checking that the value of a `field` satisfies the given `value`. File name predicates work ahead by limiting the CSV files in the collection that are queried in the first place. They extract matches for the given regex from the file names in the collection and check if they satisfy the given `value`. Any row predicates are then run only on the records in the files that satisfy all the file name predicates.

### POST `/api/{version}/render`
  
The request body is given as bytes of a CSV file. Returns a `header` and `rows`.

### POST `/api/{version}/create/{collection}`

Takes a `filename`, `header`, and `rows`. Creates a new CSV with `filename` in the given `collection`.

### DELETE `/api/{version}/delete/{collection}/{filename}`

Deletes the CSV with `filename` in the given `collection`, if it exists.


## Usage

A `Dockerfile` is provided to create a Docker image of the application. The following are some example Docker commands to get started.

```sh
docker build -t zenithds .
docker run -d -p "8750:8750" -v /path/to/storage:/data --name zenithds1 zenithds
```

Environment variables can be configured in a file `.env` and included at runtime, for example, by adding `--env-file .env` to the `docker run` command above. An example `.env` file:

```sh
ZENITHDS_NUM_WORKERS=4
ZENITHDS_DEFAULT_PAGE=0
ZENITHDS_DEFAULT_PAGE_SIZE=10
ZENITHDS_HOST=0.0.0.0
ZENITHDS_PORT=8750
# If set, prepends /zenithds before /api in the resource paths
ZENITHDS_USE_PREFIX=
```

## Development

The documentation will be revised over time.
