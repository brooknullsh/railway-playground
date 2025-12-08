FROM rust:alpine AS builder
WORKDIR /app

RUN apk add --no-cache musl-dev
RUN cargo init --bin --vcs none

COPY Cargo* .
COPY src    ./src

RUN cargo build -r

FROM scratch

COPY --from=builder /app/target/release/railway-playground .

CMD ["/railway-playground"]
