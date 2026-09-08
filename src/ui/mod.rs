pub mod app_core;
#[path = "app.rs"]
pub mod app_integration;

pub mod app {
    pub use super::app_integration::RammapApp;
}
pub mod theme;
pub mod types;
