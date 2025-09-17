FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner

COPY ./Cargo.toml ./Cargo.lock ./
RUN mkdir -p ./src && echo "fn main() {}" > ./src/main.rs

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /app/recipe.json ./recipe.json

RUN apt-get update && apt-get install -y mold cmake
ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"

RUN cargo chef cook --release --recipe-path recipe.json

COPY ./src ./src
COPY ./Cargo.toml ./Cargo.lock ./
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/simple-proxy /app/simple-proxy
ENTRYPOINT ["./simple-proxy"]
