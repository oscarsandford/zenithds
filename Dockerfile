FROM rust:1.84 AS build

WORKDIR /usr/src/zenithds
COPY Cargo.toml .
COPY ./src ./src
# Creates lockfile
RUN cargo update
# Build with optimizations
RUN cargo build --release

# Copy to reduce final image size
FROM debian:bookworm-slim

COPY --from=build /usr/src/zenithds/target/release/zenithds .
CMD ["./zenithds"]