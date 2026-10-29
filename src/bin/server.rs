use clap::Parser;
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "mysql",
    feature = "redis"
))]
use rhino::RhinoServer;
#[cfg(feature = "mysql")]
use rhino::{MysqlBackend, MysqlConfig};
#[cfg(feature = "postgres")]
use rhino::{PostgresBackend, PostgresConfig};
#[cfg(feature = "redis")]
use rhino::{RedisBackend, RedisConfig};
#[cfg(feature = "sqlite")]
use rhino::{SqliteBackend, SqliteConfig};
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "rhino-server",
    about = "etcd-compatible gRPC server backed by SQL"
)]
struct Args {
    /// gRPC listen address
    #[arg(long, default_value = "0.0.0.0:2379")]
    listen_address: String,

    /// Storage endpoint. Use a file path for SQLite (default),
    /// postgres:// for PostgreSQL, or mysql:// for MySQL.
    #[arg(long, default_value = "./db/state.db")]
    endpoint: String,

    /// Compaction interval in seconds (0 to disable)
    #[arg(long, default_value = "300")]
    compact_interval: u64,

    /// Minimum number of recent revisions to retain during compaction
    #[arg(long, default_value = "1000")]
    compact_retention: i64,

    /// Maximum database connection pool size
    #[arg(long, default_value = "10")]
    max_connections: u32,

    /// Watch progress notify interval in seconds
    #[arg(long, default_value = "5")]
    watch_progress_notify_interval: u64,

    /// Emulated etcd version string returned by the Status RPC
    #[arg(long, default_value = "3.5.13")]
    emulated_etcd_version: String,

    /// Serve Prometheus compaction gauges (rhino_current_revision,
    /// rhino_compact_revision, rhino_compaction_backlog) at /metrics on this
    /// address, e.g. 0.0.0.0:2112. Empty (default) disables the endpoint.
    #[arg(long, default_value = "")]
    metrics_bind_address: String,
}

#[tokio::main]
#[cfg_attr(
    not(any(
        feature = "sqlite",
        feature = "postgres",
        feature = "mysql",
        feature = "redis"
    )),
    allow(unused_variables)
)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    let compact_interval = Duration::from_secs(args.compact_interval);
    let notify_interval = Duration::from_secs(args.watch_progress_notify_interval);

    let endpoint = args.endpoint.as_str();
    let is_postgres = endpoint.starts_with("postgres://") || endpoint.starts_with("postgresql://");
    let is_redis = endpoint.starts_with("redis://") || endpoint.starts_with("rediss://");
    let is_mysql = endpoint.starts_with("mysql://") || endpoint.starts_with("mariadb://");

    if is_postgres {
        #[cfg(feature = "postgres")]
        {
            let config = PostgresConfig {
                dsn: args.endpoint,
                compact_interval,
                compact_min_retain: args.compact_retention,
                max_connections: args.max_connections,
                ..Default::default()
            };
            let backend = PostgresBackend::new(config).await?;
            return RhinoServer::new(backend)
                .with_notify_interval(notify_interval)
                .with_emulated_etcd_version(args.emulated_etcd_version.clone())
                .with_metrics_address(args.metrics_bind_address.clone())
                .serve(&args.listen_address)
                .await;
        }
        #[cfg(not(feature = "postgres"))]
        return Err(not_compiled("postgres"));
    }

    if is_redis {
        #[cfg(feature = "redis")]
        {
            let config = RedisConfig {
                dsn: args.endpoint,
                compact_interval,
                compact_min_retain: args.compact_retention,
                ..Default::default()
            };
            let backend = RedisBackend::new(config).await?;
            return RhinoServer::new(backend)
                .with_notify_interval(notify_interval)
                .with_emulated_etcd_version(args.emulated_etcd_version.clone())
                .with_metrics_address(args.metrics_bind_address.clone())
                .serve(&args.listen_address)
                .await;
        }
        #[cfg(not(feature = "redis"))]
        return Err(not_compiled("redis"));
    }

    if is_mysql {
        #[cfg(feature = "mysql")]
        {
            let config = MysqlConfig {
                dsn: args.endpoint,
                compact_interval,
                compact_min_retain: args.compact_retention,
                max_connections: args.max_connections,
                ..Default::default()
            };
            let backend = MysqlBackend::new(config).await?;
            return RhinoServer::new(backend)
                .with_notify_interval(notify_interval)
                .with_emulated_etcd_version(args.emulated_etcd_version.clone())
                .with_metrics_address(args.metrics_bind_address.clone())
                .serve(&args.listen_address)
                .await;
        }
        #[cfg(not(feature = "mysql"))]
        return Err(not_compiled("mysql"));
    }

    #[cfg(feature = "sqlite")]
    {
        let config = SqliteConfig {
            dsn: args.endpoint,
            compact_interval,
            compact_min_retain: args.compact_retention,
            max_connections: args.max_connections,
            ..Default::default()
        };
        let backend = SqliteBackend::new(config).await?;
        RhinoServer::new(backend)
            .with_notify_interval(notify_interval)
            .with_emulated_etcd_version(args.emulated_etcd_version.clone())
            .with_metrics_address(args.metrics_bind_address.clone())
            .serve(&args.listen_address)
            .await
    }
    #[cfg(not(feature = "sqlite"))]
    Err(not_compiled("sqlite"))
}

/// Runtime error for an endpoint whose backend was not compiled in.
#[allow(dead_code)]
fn not_compiled(backend: &str) -> Box<dyn std::error::Error> {
    format!(
        "the {backend} backend is not compiled into this rhino-server; \
         rebuild with `--features {backend}`"
    )
    .into()
}
