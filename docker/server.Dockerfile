FROM rust:alpine
WORKDIR /app

RUN apk add --no-cache musl-dev

COPY Cargo* .
COPY src    ./src

RUN cargo install cargo-watch

CMD ["cargo", "watch", "-x", "run"]
