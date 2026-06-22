FROM rust:1.90-bookworm AS builder

WORKDIR /build

# Install protobuf compiler (needed by tonic-build)
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Copy all source and build
COPY . .
RUN cargo build --release --bin rhino-server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/rhino-server /usr/local/bin/rhino-server

RUN mkdir -p /data/db

EXPOSE 2379

ENTRYPOINT ["rhino-server"]
CMD ["--listen-address", "0.0.0.0:2379", "--db-path", "/data/db/state.db"]
