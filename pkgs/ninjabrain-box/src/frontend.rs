//! The part of the overlay that is on screen.
//!
//! Everything here talks to the compositor: a layer-shell surface on the
//! overlay layer, the table drawn into its buffer, the clipboard the game's
//! F3+C lands on, and the focus tracking that decides whether any of it should
//! be visible. Nothing here knows the bot exists -- it is handed a
//! [`Stronghold`](crate::model::Stronghold) and asked to draw it.

pub mod app;
pub mod clipboard;
pub mod render;
pub mod text;
pub mod toplevel;

pub use app::App;
pub use text::Text;
