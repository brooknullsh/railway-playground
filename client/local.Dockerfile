FROM oven/bun:latest

WORKDIR /usr/src/app

COPY . .

RUN bun install --frozen-lockfile --verbose

#  3000: SvelteKit development.
# 24678: Vite HMR.
EXPOSE 3000
EXPOSE 24678

CMD ["bun", "dev", "--host", "0.0.0.0"]
