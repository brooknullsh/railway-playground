FROM golang:latest AS builder
WORKDIR /server

# Copy source files for building the binary.
COPY go.mod go.sum main.go ./
COPY internal/ ./internal

RUN CGO_ENABLED=0 GOOS=linux go build -o railway-playground

FROM scratch

COPY --from=builder /server/railway-playground .

CMD ["/railway-playground"]
