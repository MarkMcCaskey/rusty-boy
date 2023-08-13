//! IO related functions

#[cfg(feature = "desktop")]
pub mod applicationsettings;
pub mod applicationstate;
#[cfg(feature = "cli")]
pub mod arguments;
pub mod constants;
pub mod deferred_renderer;
pub mod deferred_renderer_gba;
#[cfg(feature = "desktop")]
pub mod dr_sdl2;
pub mod events;
pub mod graphics;
#[cfg(feature = "desktop")]
pub mod sound;
