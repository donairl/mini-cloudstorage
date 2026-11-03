FROM node:24-slim AS web
WORKDIR /src
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

FROM rust:1-bookworm AS api
WORKDIR /src
# cache deps: build against a stub main first
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release && rm -rf src
COPY backend/src ./src
COPY backend/migrations ./migrations
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 app \
 && mkdir -p /data/files && chown -R app:app /data
COPY --from=api /src/target/release/minidrive /app/minidrive
COPY --from=web /src/dist /app/dist
ENV STATIC_DIR=/app/dist \
    DATABASE_URL=sqlite:///data/app.db?mode=rwc \
    STORAGE_DIR=/data/files \
    PORT=3000
USER app
VOLUME /data
EXPOSE 3000
CMD ["/app/minidrive"]
