# ---- Stage 1: build the frontend ----
FROM node:22-alpine AS frontend
RUN corepack enable
WORKDIR /app/wrongint-frontend
COPY wrongint-frontend/ ./
RUN yarn install --immutable && yarn build

# ---- Stage 2: build the backend with the frontend embedded ----
FROM rustlang/rust:nightly-bookworm AS backend
WORKDIR /app
COPY wrongint-backend/ ./wrongint-backend/
# rust-embed reads ../wrongint-frontend/dist relative to the backend crate.
COPY --from=frontend /app/wrongint-frontend/dist ./wrongint-frontend/dist
WORKDIR /app/wrongint-backend
RUN cargo build --release --features embed-frontend

# ---- Stage 3: runtime ----
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=backend /app/wrongint-backend/target/release/wrongint-backend /usr/local/bin/wrongint-backend
RUN mkdir -p /data
VOLUME /data
EXPOSE 8080
# Config is provided at runtime (mount it at /config.toml).
CMD ["wrongint-backend", "run", "/config.toml"]