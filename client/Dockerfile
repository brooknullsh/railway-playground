FROM oven/bun:latest AS builder

WORKDIR /usr/src/app

COPY . .

RUN bun install --frozen-lockfile --verbose
RUN bun run build

FROM oven/bun:distroless

COPY --from=builder /usr/src/app/build ./build

CMD ["build/index.js"]
