pub mod renderer;
#[cfg(feature = "desktop")]
pub mod sdl2;
#[cfg(all(feature = "vulkan", feature = "desktop"))]
pub mod vulkan;

/*
#[cfg(not(feature = "vulkan"))]
pub mod vulkan {
    impl VulkanRenderer {
        pub fn new(app_settings: &ApplicationSettings) -> Result<Self, String> {
    }
}
*/
