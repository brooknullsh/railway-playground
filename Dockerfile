FROM rust:alpine AS builder
WORKDIR /app

ARG LOG_LEVEL
ARG DATABASE_URL
ARG PORT
ARG ACCESS_SECRET
ARG JWT_SECRET

RUN apk add --no-cache musl-dev
RUN cargo init --bin --vcs none

COPY Cargo* .
COPY src    ./src

RUN cargo build -r

FROM scratch

COPY --from=builder /app/target/release/railway-playground .

CMD ["/railway-playground"]
