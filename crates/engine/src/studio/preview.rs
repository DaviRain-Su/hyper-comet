//! Local HTTP server for Studio dapp HTML previews.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};

#[derive(Clone, Default)]
pub struct StudioPreview {
    inner: Arc<Mutex<Option<ActivePreview>>>,
}

struct ActivePreview {
    url: String,
    module: String,
    address: String,
    shutdown: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Clone)]
pub struct PreviewStatus {
    pub url: String,
    pub module: String,
    pub address: String,
}

impl StudioPreview {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn status(&self) -> Option<PreviewStatus> {
        let guard = self.inner.lock().await;
        guard.as_ref().map(|active| PreviewStatus {
            url: active.url.clone(),
            module: active.module.clone(),
            address: active.address.clone(),
        })
    }

    pub async fn stop(&self) {
        let mut guard = self.inner.lock().await;
        if let Some(mut active) = guard.take()
            && let Some(tx) = active.shutdown.take()
        {
            let _ = tx.send(());
        }
    }

    /// Replace any running preview with a new HTML body on an ephemeral port.
    pub async fn start(
        &self,
        html: String,
        module: String,
        address: String,
    ) -> Result<PreviewStatus, String> {
        self.stop().await;

        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .map_err(|e| format!("bind preview server: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("preview local_addr: {e}"))?
            .port();
        let url = format!("http://127.0.0.1:{port}/");
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let body = Arc::new(html);

        tokio::spawn(serve_loop(listener, body, shutdown_rx));

        let status = PreviewStatus {
            url: url.clone(),
            module: module.clone(),
            address: address.clone(),
        };
        *self.inner.lock().await = Some(ActivePreview {
            url,
            module,
            address,
            shutdown: Some(shutdown_tx),
        });
        Ok(status)
    }
}

async fn serve_loop(
    listener: TcpListener,
    body: Arc<String>,
    mut shutdown: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            accepted = listener.accept() => {
                match accepted {
                    Ok((mut stream, _)) => {
                        let body = Arc::clone(&body);
                        tokio::spawn(async move {
                            let mut buf = [0u8; 2048];
                            let _ = stream.read(&mut buf).await;
                            let bytes = body.as_bytes();
                            let header = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 Content-Type: text/html; charset=utf-8\r\n\
                                 Content-Length: {}\r\n\
                                 Cache-Control: no-store\r\n\
                                 Connection: close\r\n\
                                 Access-Control-Allow-Origin: *\r\n\
                                 \r\n",
                                bytes.len()
                            );
                            let _ = stream.write_all(header.as_bytes()).await;
                            let _ = stream.write_all(bytes).await;
                            let _ = stream.shutdown().await;
                        });
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn preview_serves_html_and_stops() {
        let preview = StudioPreview::new();
        let status = preview
            .start(
                "<html>ok</html>".into(),
                "Mod".into(),
                "0x1".into(),
            )
            .await
            .unwrap();
        assert!(status.url.starts_with("http://127.0.0.1:"));

        let client = reqwest::Client::new();
        let text = client
            .get(&status.url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert_eq!(text, "<html>ok</html>");

        preview.stop().await;
        assert!(preview.status().await.is_none());
    }
}
