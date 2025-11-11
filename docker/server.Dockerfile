FROM rust:latest AS builder
WORKDIR /server-local

# Copy configuration and source files.
COPY Cargo* .
COPY src    ./src

# Add hot-reloading with mounted source files.
RUN cargo install cargo-watch

CMD ["cargo", "watch", "-x", "run"]
