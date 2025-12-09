FROM golang:alpine AS builder
WORKDIR /app

COPY go.mod go.sum main.go ./
COPY internal/             ./internal

RUN go install github.com/air-verse/air@latest
RUN go mod download

CMD ["air"]
