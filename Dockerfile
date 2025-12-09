FROM golang:alpine AS builder
WORKDIR /app

COPY go.mod go.sum main.go ./
COPY internal/             ./internal

RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o server

FROM scratch

COPY --from=builder /app/server .

CMD ["/server"]
