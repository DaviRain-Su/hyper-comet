//! Dedicated Preview WebView window launcher.
//!
//! gpui cannot host an in-pane child WebView reliably (especially on Linux /
//! Wayland). Studio opens a **sibling process of the same `comet` binary**:
//! `comet preview-webview <url>`. No second product executable to ship.
//!
//! Fallback: Chromium / Chrome / Brave `--app=<url>` when wry/WebKit is missing.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
pub struct StudioWebView {
    child: Arc<Mutex<Option<Child>>>,
}

impl StudioWebView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_running(&self) -> bool {
        let mut guard = self.child.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(None) => true,
                Ok(Some(_)) => {
                    *guard = None;
                    false
                }
                Err(_) => {
                    *guard = None;
                    false
                }
            },
            None => false,
        }
    }

    /// Open (or replace) the Preview WebView for `url`.
    pub fn open(&self, url: &str) -> Result<WebViewBackend, String> {
        let url = url.trim();
        if url.is_empty() {
            return Err("preview URL is empty — Start preview first".into());
        }
        self.stop();

        // Prefer re-exec of this app: `comet preview-webview <url>`.
        if let Ok(exe) = std::env::current_exe() {
            match Command::new(&exe)
                .arg("preview-webview")
                .arg(url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(child) => {
                    *self.child.lock().unwrap_or_else(|e| e.into_inner()) = Some(child);
                    return Ok(WebViewBackend::CometSelf);
                }
                Err(err) => {
                    tracing::warn!(error = %err, "comet preview-webview spawn failed");
                }
            }
        }

        // Explicit override still allowed for packaging experiments.
        if let Some(bin) = resolve_explicit_webview() {
            let child = Command::new(&bin)
                .arg(url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("spawn {}: {e}", bin.display()))?;
            *self.child.lock().unwrap_or_else(|e| e.into_inner()) = Some(child);
            return Ok(WebViewBackend::ExplicitBinary);
        }

        if let Some(browser) = resolve_chromium_family() {
            let mut cmd = Command::new(&browser);
            cmd.arg(format!("--app={url}"))
                .arg("--new-window")
                .arg("--window-size=960,780")
                .arg(format!(
                    "--user-data-dir={}",
                    std::env::temp_dir()
                        .join("proofship-preview-webview")
                        .display()
                ))
                .arg("--no-first-run")
                .arg("--no-default-browser-check")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            let child = cmd
                .spawn()
                .map_err(|e| format!("spawn {}: {e}", browser.display()))?;
            *self.child.lock().unwrap_or_else(|e| e.into_inner()) = Some(child);
            return Ok(WebViewBackend::ChromiumApp);
        }

        Err(
            "No WebView backend found. Build comet with WebKitGTK (Linux: libwebkit2gtk-4.1-dev), or install Chromium."
                .into(),
        )
    }

    pub fn stop(&self) {
        let mut guard = self.child.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for StudioWebView {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebViewBackend {
    /// `comet preview-webview` — same shipped binary.
    CometSelf,
    ExplicitBinary,
    ChromiumApp,
}

impl WebViewBackend {
    pub fn label(self) -> &'static str {
        match self {
            Self::CometSelf => "comet preview-webview",
            Self::ExplicitBinary => "PROOFSHIP_WEBVIEW",
            Self::ChromiumApp => "Chromium app window",
        }
    }
}

fn resolve_explicit_webview() -> Option<PathBuf> {
    let explicit = std::env::var("PROOFSHIP_WEBVIEW").ok()?;
    let p = PathBuf::from(explicit.trim());
    if !p.as_os_str().is_empty() && is_executable(&p) {
        Some(p)
    } else {
        None
    }
}

fn resolve_chromium_family() -> Option<PathBuf> {
    const CANDIDATES: &[&str] = &[
        "chromium",
        "chromium-browser",
        "google-chrome",
        "google-chrome-stable",
        "brave-browser",
        "microsoft-edge",
        "msedge",
    ];
    for name in CANDIDATES {
        if let Some(path) = which(name) {
            return Some(path);
        }
    }
    None
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|p| is_executable(p))
}

fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_labels_are_stable() {
        assert_eq!(WebViewBackend::CometSelf.label(), "comet preview-webview");
        assert_eq!(WebViewBackend::ChromiumApp.label(), "Chromium app window");
    }

    #[test]
    fn open_rejects_empty_url() {
        let wv = StudioWebView::new();
        assert!(wv.open("").is_err());
        assert!(wv.open("   ").is_err());
    }
}
