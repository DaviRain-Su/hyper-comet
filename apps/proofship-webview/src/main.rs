//! ProofShip Preview WebView — dedicated OS window via wry/WebKit.
//!
//! Usage:
//!   proofship-webview http://127.0.0.1:PORT/
//!
//! Spawned by Studio Preview when available; falls back to Chromium `--app=`
//! if this binary is missing.

use std::env;

use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

fn main() {
    let url = env::args()
        .nth(1)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            eprintln!("usage: proofship-webview <url>");
            std::process::exit(64);
        });

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("ProofShip Preview")
        .with_inner_size(tao::dpi::LogicalSize::new(960.0, 780.0))
        .build(&event_loop)
        .expect("create window");

    let _webview = WebViewBuilder::new()
        .with_url(&url)
        .build(&window)
        .expect("create webview");

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
