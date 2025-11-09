FROM rust:latest AS builder

# Create a shell project.
RUN cargo new --bin rust-playground --vcs none
WORKDIR /rust-playground

# Copy the manifest files (Cargo.toml & Cargo.lock).
COPY ./Cargo* .

# Build dependencies and remove shell source files.
RUN cargo build -r
RUN rm ./src/*.rs

# Copy working source files into shell project.
COPY ./src ./src

# Remove incremental artifacts and rebuild.
RUN rm -f ./target/release/deps/rust_playground*
RUN cargo build -r

# Lightweight final base image.
FROM debian:bookworm-slim

# Copy only the executable from the rust image.
COPY --from=builder /rust-playground/target/release/rust-playground /rust-playground

CMD ["/rust-playground"]
