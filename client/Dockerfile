FROM oven/bun:latest AS builder
WORKDIR /app

COPY package.json     .
COPY vite.config.ts   .
COPY svelte.config.js .
COPY src              ./src

RUN bun install --verbose
RUN bun run build

FROM oven/bun:distroless

COPY --from=builder /app/build /build

CMD ["/build/index.js"]
