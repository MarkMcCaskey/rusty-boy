pub mod sdl2;
#[cfg(feature = "vulkan")]
pub mod vulkan;
pub mod renderer;

/*
#[cfg(not(feature = "vulkan"))]
pub mod vulkan {
    impl VulkanRenderer {
        pub fn new(app_settings: &ApplicationSettings) -> Result<Self, String> {
    }
}
*/
