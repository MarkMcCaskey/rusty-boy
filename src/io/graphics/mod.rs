#[cfg(feature = "opengl")]
pub mod gl;
pub mod renderer;
pub mod sdl2;
#[cfg(feature = "vulkan")]
pub mod vulkan;

/*
#[cfg(not(feature = "vulkan"))]
pub mod vulkan {
    impl VulkanRenderer {
        pub fn new(app_settings: &ApplicationSettings) -> Result<Self, String> {
    }
}
*/
