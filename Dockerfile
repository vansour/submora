FROM node:24-trixie AS web-assets
WORKDIR /app/web

COPY web/package.json web/package-lock.json ./
RUN npm ci

COPY web/ ./
RUN npm run build

FROM rust:trixie AS app-binary
WORKDIR /app

# 先复制依赖清单，利用 Docker 层缓存
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml backend/

# 创建 dummy main.rs 触发依赖下载
RUN mkdir -p backend/src && \
    echo "fn main() {}" > backend/src/main.rs && \
    echo "pub fn dummy() {}" > backend/src/lib.rs

# 构建依赖（会被缓存）
RUN cargo build --release -p submora

# 删除 dummy 文件和与 submora 相关的指纹缓存，避免真实源码被旧的占位 lib 复用
RUN rm -rf \
      backend/src \
      target/release/deps/submora* \
      target/release/submora* \
      target/release/.fingerprint/submora*

# 复制真实源码
COPY backend/src backend/src
COPY backend/migrations backend/migrations
COPY backend/tests backend/tests

# 构建真实二进制（只编译变更的代码）
RUN cargo build --release -p submora && strip target/release/submora

FROM debian:trixie-slim

RUN apt update && \
    DEBIAN_FRONTEND=noninteractive apt install -y --no-install-recommends \
      ca-certificates \
      curl \
      libc-bin \
      libc6 \
      sqlite3 \
      tzdata && \
    apt clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

ENV TZ=Asia/Shanghai \
    RUST_LOG=info \
    RUST_BACKTRACE=1 \
    HOST=0.0.0.0 \
    PORT=8080 \
    WEB_DIST_DIR=/app/dist \
    DATABASE_URL=sqlite:///app/data/substore.db?mode=rwc

RUN ln -snf /usr/share/zoneinfo/Asia/Shanghai /etc/localtime && echo "Asia/Shanghai" > /etc/timezone

WORKDIR /app
RUN mkdir -p /app/data /app/dist && chmod 777 /app/data

COPY --from=app-binary /app/target/release/submora /app/submora
COPY --from=web-assets /app/web/dist /app/dist

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD /usr/bin/curl -f http://localhost:8080/healthz || exit 1

EXPOSE 8080
CMD ["/app/submora"]
