FROM rust:latest AS builder
WORKDIR /server

# Create a new project shell.
RUN cargo init --bin --vcs none

# Overwrite configuration and source files.
COPY Cargo* .
COPY src    ./src

# Generate a release build using the generated/copied files.
RUN cargo build -r

FROM debian:bookworm-slim

# NOTE: Binary name is defined within the copied Cargo.toml file, rather than
# the generated shell.
COPY --from=builder /server/target/release/railway-playground /railway-playground

CMD ["/railway-playground"]
