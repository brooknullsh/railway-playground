FROM oven/bun:latest
WORKDIR /app

COPY client/package.json     .
COPY client/vite.config.ts   .
COPY client/svelte.config.js .
COPY client/src              ./src

RUN bun install --verbose

# 3000  : SvelteKit
# 24678 : Vite
EXPOSE 3000
EXPOSE 24678

CMD ["bun", "dev", "--host", "0.0.0.0"]
