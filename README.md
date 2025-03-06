# zenithds

ZenithDS is a data service for querying CSV data. This software is designed to be configured and extended based on domain use cases.

## Design

Each directory in `/data` is considered a collection (for example, `/data/main`), and all the files inside a given directory in `/data` are assumed to be CSV files. There is currently no way to create a collection through the API; the directory needs to be created on the file system manually.

The header of a CSV file in a collection is considered the first row that has a complete set of values (that is, no empty slots). Currently, the program assumes that, in a given collection, each CSV file has the same header. Therefore it is suggested to use the API for creating files in the collection. However, one can place files directly in the collection directory in the file system, ensuring their headers are consistent. Inconsistent headers in a collection can produce inconsistent behaviour.

A `Dockerfile` is provided to create a Docker image of the application. The following are some example Docker commands to get started. Instead of mounting one directory to `/data` as below, one can mount to `/data/main` directly, for example, and can mount multiple collections in this way.

```sh
docker build -t zenithds .
docker run -d -p "8750:8750" -v /path/to/storage:/data --name zenithds1 zenithds
```

Environment variables can be included at runtime to configure the data service. If not set, its default value will be used.

```sh
ZENITHDS_NUM_WORKERS=4
ZENITHDS_DEFAULT_PAGE=0
ZENITHDS_DEFAULT_PAGE_SIZE=10
ZENITHDS_HOST=0.0.0.0
ZENITHDS_PORT=8750
# If set, prepends /zenithds before /api in the resource paths
ZENITHDS_USE_PREFIX=
# The list of options to set for Access-Control-Allow-Origin header, separated by commas
ZENITHDS_ALLOWED_ORIGINS=
```

## Endpoints

The data service currently supports a REST API. Some of the names may change.

In this section, unless otherwise noted, a `header` is a list of field/column names as strings, and `rows` is a list of lists with values as strings.

#### POST `/api/{version}/query/{collection}`
  
Queries the data in a `collection`. Takes `predicates` that can influence the rows returned, and `fields` which influence the fields/columns returned. Returns a `header` and `rows`.

The `fields` are simply strings matching the names of the fields. The `predicates` are strings, and can be in one of two forms:

- Row-level predicates: `field OP value`, where `OP` can be any operator recognized by the program
- File name predicates: `HAS regex OP value`, where `regex` is a regular expression

Row-level predicates limit which rows are returned by checking that the value of a `field` in a row satisfies the given `value`. File name predicates work ahead by limiting the CSV files in the collection that are queried in the first place. They extract matches for the given regex from the file names in the collection and check if they satisfy the given `value`. Any row predicates are then run only on the records in the files that satisfy all the file name predicates.

The rows are currently returned in a nondeterministic order.

#### POST `/api/{version}/render`
  
The request body is given as bytes of a CSV file. Returns a `header` and `rows`.

#### POST `/api/{version}/create/{collection}`

Takes a `filename`, `header`, and `rows`. Creates a new CSV with `filename` in the given `collection`.

#### DELETE `/api/{version}/delete/{collection}/{filename}`

Deletes the CSV with `filename` in the given `collection`, if it exists.

<hr>

## Development

The documentation will be revised over time.
