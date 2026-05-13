# Rhino

**etcd, backed by your database.**

Rhino is a drop-in etcd v3 gRPC server written in Rust that stores everything in a relational database or Redis. Same API your tools already speak — simpler operations, no Raft consensus required.

## Why Rhino?

Running etcd in production means managing a distributed consensus cluster: quorum maintenance, defragmentation, backup/restore choreography, and peer TLS. For many workloads — edge deployments, single-node clusters, CI environments, development — that complexity isn't justified.

Rhino eliminates it. Your etcd clients connect to Rhino exactly as they would to etcd. Under the hood, every key-value mutation becomes a row in your database or a Redis data structure. You get the operational model of a database you already know (backup with `pg_dump`, replicate with your existing tooling, inspect state with `SELECT *` or `redis-cli`) while keeping full etcd v3 API compatibility.

## Features

- **Full etcd v3 gRPC API** — KV, Watch, Lease, and Maintenance services
- **Atomic transactions** — compare-and-swap with revision-based optimistic concurrency
- **Watch streams** — real-time gRPC streaming of key changes with prefix matching and historical replay
- **Revision history** — log-structured storage with monotonic revisions; query any point in time
- **Range queries** — list and count keys by prefix with pagination
- **Auto-compaction** — background compaction removes old revisions on a configurable schedule
- **Pluggable backends** — trait-based abstraction lets you swap storage engines
- **Async-first** — built on Tokio and Tonic with non-blocking I/O throughout

## Supported Backends

| Backend    | Status  |
|------------|---------|
| SQLite     | Ready   |
| PostgreSQL | Ready   |
| MySQL      | Ready   |
| Redis      | Ready   |

## Quickstart

### As a library

Add Rhino to your project:

```sh
cargo add rhino
```

Embed with SQLite:

```rust
use rhino::{RhinoServer, SqliteBackend, SqliteConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = SqliteBackend::new(SqliteConfig::default()).await?;
    let server = RhinoServer::new(backend);
    server.serve("0.0.0.0:2379").await?;
    Ok(())
}
```

Or with PostgreSQL:

```rust
use rhino::{RhinoServer, PostgresBackend, PostgresConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = PostgresConfig {
        dsn: "postgres://user:pass@localhost/kubernetes".to_string(),
        ..Default::default()
    };

    let backend = PostgresBackend::new(config).await?;
    let server = RhinoServer::new(backend);
    server.serve("0.0.0.0:2379").await?;
    Ok(())
}
```

Or with Redis:

```rust
use rhino::{RhinoServer, RedisBackend, RedisConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RedisConfig {
        dsn: "redis://127.0.0.1:6379".to_string(),
        ..Default::default()
    };

    let backend = RedisBackend::new(config).await?;
    let server = RhinoServer::new(backend);
    server.serve("0.0.0.0:2379").await?;
    Ok(())
}
```

### As a standalone server

Build and run the included binary:

```sh
cargo run --bin rhino-server
```

The `--endpoint` flag selects the backend automatically:

```sh
# SQLite (default)
cargo run --bin rhino-server -- --endpoint ./db/state.db

# PostgreSQL
cargo run --bin rhino-server -- --endpoint postgres://user:pass@localhost/kubernetes

# MySQL
cargo run --bin rhino-server -- --endpoint mysql://root:root@localhost/kubernetes

# Redis
cargo run --bin rhino-server -- --endpoint redis://127.0.0.1:6379
```

Options:

```
--listen-address <ADDR>      gRPC listen address [default: 0.0.0.0:2379]
--endpoint <ENDPOINT>        File path for SQLite, postgres:// for PostgreSQL,
                             mysql:// for MySQL, redis:// for Redis [default: ./db/state.db]
--compact-interval <SECS>    Compaction interval in seconds, 0 to disable [default: 300]
```

Control log verbosity with `RUST_LOG`:

```sh
RUST_LOG=debug cargo run --bin rhino-server
```

### With Docker

```sh
docker build -t rhino .
docker run -p 2379:2379 -v rhino-data:/data rhino
```

The container stores its database at `/data/db/state.db`. Mount a volume to persist across restarts.

Override defaults with arguments:

```sh
docker run -p 2379:2379 rhino --listen-address 0.0.0.0:2379 --compact-interval 60
```

### With etcdctl

Once the server is running (via any method above), any standard etcd client works:

```sh
# Put a key
etcdctl put /myapp/config '{"port": 8080}'

# Read it back
etcdctl get /myapp/config

# List by prefix
etcdctl get /myapp/ --prefix

# Watch for changes
etcdctl watch /myapp/ --prefix
```

## Configuration

### SqliteConfig

| Field                | Type       | Default          | Description                            |
|----------------------|------------|------------------|----------------------------------------|
| `dsn`                | `String`   | `./db/state.db`  | Path to the SQLite database file       |
| `compact_interval`   | `Duration` | 300 seconds      | How often to run background compaction |
| `compact_min_retain` | `i64`      | 1000             | Minimum revisions to keep              |
| `compact_batch_size` | `i64`      | 1000             | Rows processed per compaction batch    |

### PostgresConfig

| Field                | Type       | Default                                          | Description                            |
|----------------------|------------|--------------------------------------------------|----------------------------------------|
| `dsn`                | `String`   | `postgres://postgres:postgres@localhost/kubernetes` | PostgreSQL connection string           |
| `compact_interval`   | `Duration` | 300 seconds                                      | How often to run background compaction |
| `compact_min_retain` | `i64`      | 1000                                             | Minimum revisions to keep              |
| `compact_batch_size` | `i64`      | 1000                                             | Rows processed per compaction batch    |
| `max_connections`    | `u32`      | 5                                                | Maximum connections in the pool        |

### MysqlConfig

| Field                | Type       | Default                              | Description                            |
|----------------------|------------|--------------------------------------|----------------------------------------|
| `dsn`                | `String`   | `mysql://root@localhost/kubernetes`   | MySQL connection string                |
| `compact_interval`   | `Duration` | 300 seconds                          | How often to run background compaction |
| `compact_min_retain` | `i64`      | 1000                                 | Minimum revisions to keep              |
| `compact_batch_size` | `i64`      | 1000                                 | Rows processed per compaction batch    |
| `max_connections`    | `u32`      | 5                                    | Maximum connections in the pool        |

### RedisConfig

| Field                | Type       | Default                      | Description                            |
|----------------------|------------|------------------------------|----------------------------------------|
| `dsn`                | `String`   | `redis://127.0.0.1:6379`    | Redis connection URL                   |
| `compact_interval`   | `Duration` | 300 seconds                  | How often to run background compaction |
| `compact_min_retain` | `i64`      | 1000                         | Minimum revisions to keep              |
| `compact_batch_size` | `i64`      | 1000                         | Rows processed per compaction batch    |

Set `compact_interval` to `Duration::ZERO` to disable automatic compaction on any backend.

## Documentation

- **[Getting Started](docs/GETTING_STARTED.md)** — installation, first steps, and common usage patterns
- **[Architecture](docs/ARCHITECTURE.md)** — system design, data model, and how the pieces fit together
- **[Testing](docs/TESTING.md)** — how to run tests, write new ones, and smoke-test with `etcdctl`

## Running Tests

```sh
cargo test
```

This runs the 16 SQLite backend tests using temporary databases — no external services needed. To also run the PostgreSQL, MySQL, or Redis tests, provide connection details:

```sh
# PostgreSQL
RHINO_POSTGRES_DSN="postgres://postgres:postgres@localhost/rhino_test" cargo test

# Redis (runs by default at localhost:6379; set SKIP_REDIS_TESTS=1 to skip)
cargo test --test redis_backend -- --test-threads=1
```

See **[docs/TESTING.md](docs/TESTING.md)** for the full testing guide: prerequisites, test inventory, how to write new tests, and smoke-testing with `etcdctl`.

## License

Apache 2.0 — see [LICENSE](LICENSE).
