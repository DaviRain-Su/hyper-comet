//! Dedicated Preview WebView window (`comet preview-webview <url>`).
//!
//! Same binary as the headed app — Studio re-execs itself so we do not ship a
//! second product executable. gpui cannot host a reliable in-pane child
//! WebView on Linux/Wayland, so this is a managed sibling OS window.

use anyhow::{Context, Result};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

pub fn run(url: &str) -> Result<()> {
    let url = url.trim();
    anyhow::ensure!(!url.is_empty(), "url must not be empty");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("ProofShip Preview")
        .with_inner_size(tao::dpi::LogicalSize::new(960.0, 780.0))
        .build(&event_loop)
        .context("create preview window")?;

    let _webview = WebViewBuilder::new()
        .with_url(url)
        .build(&window)
        .context("create webview (need WebKitGTK on Linux: libwebkit2gtk-4.1)")?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}
