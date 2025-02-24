# zenithds

ZenithDS is a data service using CSV for data storage. This software is designed to be configured and extended based on domain use cases.

## Endpoints

The data service currently supports a REST API.

### GET `/api/{version}/query/{collection}`

A simple way of querying the data in a `collection`. Returns a `header` (a list of fields/columns as strings) and `rows` (a list of lists with values as strings). Optional query parameters can influence the number of rows returned.

### POST `/api/{version}/query/{collection}`
  
A more complex way of querying the data in a `collection`. Takes `predicates` that can influence the rows returned, and `fields` which influences the fields/columns returned. Returns a `header` (a list of fields/columns as strings) and `rows` (a list of lists with values as strings).

The `fields` are simply strings matching the names of the fields. The `predicates` are in the form `field == value` where `==` can be any operator recognized by the program.

### POST `/api/{version}/upload`
  
Takes bytes that will be parsed as CSV data. Returns a `header` (a list of fields/columns as strings), `rows` (a list of lists with values as strings), and a list of row numbers that are suggested to be removed.

### POST `/api/{version}/create/{collection}`

Takes a `header` (a list of fields/columns as strings), `rows` (a list of lists with values as strings). Creates a new CSV in the given `collection` with a timestamp.

## Usage

A `Dockerfile` is provided to create a Docker image of the application. The following are some example Docker commands to get started.

```sh
docker build -t zenithds .
docker run -d --rm -p "8750:8750" -v /path/to/storage:/data --name zenithds1 zenithds
```

Environment variables can be configured in a file `.env` and included at runtime, for example, by adding `--env-file .env` to the `docker run` command above. An example `.env` file:

```sh
ZENITHDS_NUM_WORKERS=4
ZENITHDS_DEFAULT_PAGE=0
ZENITHDS_DEFAULT_PAGE_SIZE=10
HOST=0.0.0.0
PORT=8750
```

## Development

The documentation will be revised over time.
