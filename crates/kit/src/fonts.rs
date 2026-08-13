//! Embedded UI fonts — Geist and Geist Mono (variable + static weights),
//! © Vercel Inc., SIL Open Font License 1.1 (https://openfontlicense.org).
//!
//! Bundled so type ships with the binary instead of depending on the host.
//! Static weights sit alongside the variable file: gpui's cosmic-text path
//! (Linux) rasterizes variable fonts at their default instance only — it never
//! applies `wght` coordinates — so medium/semibold/bold text silently paints
//! at 400 with just the variable TTF registered. The statics give the face
//! matcher real 500/600/700 faces (macOS/CoreText applies the variable axis
//! natively and simply never falls through to these).

use std::borrow::Cow;

use gpui::App;

static FONT_GEIST: &[u8] = include_bytes!("../assets/fonts/Geist.ttf");
static FONT_GEIST_MONO: &[u8] = include_bytes!("../assets/fonts/GeistMono.ttf");
static FONT_GEIST_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Geist-Medium.ttf");
static FONT_GEIST_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Geist-SemiBold.ttf");
static FONT_GEIST_BOLD: &[u8] = include_bytes!("../assets/fonts/Geist-Bold.ttf");

/// Register the embedded fonts with the gpui text system. Failure is non-fatal:
/// the theme's system fallbacks take over.
pub fn register_fonts(cx: &App) {
    if let Err(err) = cx.text_system().add_fonts(vec![
        Cow::Borrowed(FONT_GEIST),
        Cow::Borrowed(FONT_GEIST_MONO),
        Cow::Borrowed(FONT_GEIST_MEDIUM),
        Cow::Borrowed(FONT_GEIST_SEMIBOLD),
        Cow::Borrowed(FONT_GEIST_BOLD),
    ]) {
        tracing::warn!(error = %err, "failed to register embedded Geist fonts");
    }
}
