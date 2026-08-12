//! Dedicated Preview WebView window launcher.
//!
//! gpui cannot host an in-pane child WebView reliably (especially on Linux /
//! Wayland). Instead we open a **managed app window**:
//! 1. Prefer a bundled `proofship-webview` binary (wry) when present.
//! 2. Else Chromium / Chrome / Brave `--app=<url>` (real WebView chrome-less window).
//! 3. Else fall back to the system browser via `open_url`.

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

        if let Some(bin) = resolve_proofship_webview() {
            let child = Command::new(&bin)
                .arg(url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("spawn {}: {e}", bin.display()))?;
            *self.child.lock().unwrap_or_else(|e| e.into_inner()) = Some(child);
            return Ok(WebViewBackend::ProofshipWry);
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
            "No WebView backend found. Install Chromium/Chrome, or build apps/proofship-webview (wry)."
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
    ProofshipWry,
    ChromiumApp,
}

impl WebViewBackend {
    pub fn label(self) -> &'static str {
        match self {
            Self::ProofshipWry => "proofship-webview",
            Self::ChromiumApp => "Chromium app window",
        }
    }
}

fn resolve_proofship_webview() -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("PROOFSHIP_WEBVIEW") {
        let p = PathBuf::from(explicit.trim());
        if !p.as_os_str().is_empty() && is_executable(&p) {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        for name in ["proofship-webview", "comet-preview-webview"] {
            let candidate = dir.join(name);
            if is_executable(&candidate) {
                return Some(candidate);
            }
        }
        // Dev: `target/debug/comet` → sibling `proofship-webview`.
        let candidate = dir.join("proofship-webview");
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    // Dev fallback: CARGO_TARGET_DIR / target/{debug,release}/proofship-webview
    for profile in ["debug", "release"] {
        let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(profile)
            .join("proofship-webview");
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    which("proofship-webview")
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
        assert_eq!(WebViewBackend::ChromiumApp.label(), "Chromium app window");
        assert_eq!(WebViewBackend::ProofshipWry.label(), "proofship-webview");
    }

    #[test]
    fn open_rejects_empty_url() {
        let wv = StudioWebView::new();
        assert!(wv.open("").is_err());
        assert!(wv.open("   ").is_err());
    }
}
