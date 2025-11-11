FROM oven/bun:latest AS builder
WORKDIR /client

# Copy source files for building the client.
COPY package.json     .
COPY vite.config.ts   .
COPY svelte.config.js .
COPY src              ./src

# Generate build output after installing linux dependencies.
RUN bun install --verbose
RUN bun run build

FROM oven/bun:distroless

COPY --from=builder /client/build /build

CMD ["/build/index.js"]
