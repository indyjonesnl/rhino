pub mod proto {
    pub mod mvccpb {
        tonic::include_proto!("mvccpb");
    }
    pub mod authpb {
        tonic::include_proto!("authpb");
    }
    pub mod etcdserverpb {
        tonic::include_proto!("etcdserverpb");
    }
}

pub mod backend;
pub mod drivers;
pub mod metrics;
pub mod server;

pub use backend::{Backend, BackendError, Event, KeyValue, WatchResult};
#[cfg(feature = "mysql")]
pub use drivers::mysql::{MysqlBackend, MysqlConfig};
#[cfg(feature = "postgres")]
pub use drivers::postgres::{PostgresBackend, PostgresConfig};
#[cfg(feature = "redis")]
pub use drivers::redis::{RedisBackend, RedisConfig};
#[cfg(feature = "sqlite")]
pub use drivers::sqlite::{SqliteBackend, SqliteConfig};
pub use server::RhinoServer;
