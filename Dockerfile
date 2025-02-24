FROM rust:1.84 AS build

WORKDIR /usr/src/zenithds
COPY Cargo.toml .
# Creates lockfile
RUN cargo update
COPY ./src ./src

# For development/debugging
# CMD ["cargo", "run"]

# Build with optimizations
# RUN cargo build --release
# CMD ["target/release/zenithds"]

# TODO run the compiled application in another image