FROM golang:latest AS builder
WORKDIR /server-local

# Copy source files for running the server.
COPY go.mod go.sum main.go ./
COPY internal/             ./internal

# Add hot-reloading with mounted source files.
RUN go install github.com/air-verse/air@latest
RUN go mod download

CMD ["air"]
