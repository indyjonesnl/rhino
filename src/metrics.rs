//! Optional Prometheus-text `/metrics` endpoint exposing compaction progress.
//!
//! kine has `--metrics-bind-address` (pkg/app/app.go) but no compaction-lag
//! gauge (pkg/metrics/metrics.go), so there is no upstream metric to port;
//! the gauge names here are rhino's own. The endpoint is off by default.

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

use crate::backend::Backend;

/// Render the compaction gauges in Prometheus text exposition format.
/// The backlog is clamped at zero: the compact marker can briefly read ahead
/// of a cached head revision.
pub fn render(current_revision: i64, compact_revision: i64) -> String {
    let backlog = (current_revision - compact_revision).max(0);
    format!(
        "# HELP rhino_current_revision Head revision of the store.\n\
         # TYPE rhino_current_revision gauge\n\
         rhino_current_revision {current_revision}\n\
         # HELP rhino_compact_revision Highest revision compaction has reached.\n\
         # TYPE rhino_compact_revision gauge\n\
         rhino_compact_revision {compact_revision}\n\
         # HELP rhino_compaction_backlog Revisions not yet compacted (current - compact).\n\
         # TYPE rhino_compaction_backlog gauge\n\
         rhino_compaction_backlog {backlog}\n"
    )
}

fn response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

async fn handle<B: Backend>(backend: &B, mut stream: TcpStream) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let target = req.split_whitespace().nth(1).unwrap_or("");
    let out = if target.split('?').next() == Some("/metrics") {
        match (
            backend.current_revision().await,
            backend.compact_revision().await,
        ) {
            (Ok(head), Ok(compact)) => response("200 OK", &render(head, compact)),
            (Err(e), _) | (_, Err(e)) => response("500 Internal Server Error", &format!("{e}\n")),
        }
    } else {
        response("404 Not Found", "not found\n")
    };
    stream.write_all(out.as_bytes()).await?;
    stream.shutdown().await
}

/// Serve `/metrics` on an already-bound listener until the task is dropped.
pub async fn serve<B: Backend>(backend: Arc<B>, listener: TcpListener) {
    if let Ok(addr) = listener.local_addr() {
        info!("rhino metrics listening on {}", addr);
    }
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let backend = backend.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle(&*backend, stream).await {
                        warn!("metrics connection error: {e}");
                    }
                });
            }
            Err(e) => warn!("metrics accept error: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn renders_backlog() {
        let out = render(120, 100);
        assert!(out.contains("rhino_current_revision 120\n"));
        assert!(out.contains("rhino_compact_revision 100\n"));
        assert!(out.contains("rhino_compaction_backlog 20\n"));
    }

    #[test]
    fn backlog_never_negative() {
        assert!(render(5, 9).contains("rhino_compaction_backlog 0\n"));
    }
}
