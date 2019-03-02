use super::renderer;
use super::renderer::EventResponse;
use cpu::Cpu;
use io::applicationsettings::ApplicationSettings;
use io::graphics::renderer::Renderer;
use std::mem;
use std::sync::Arc;
use vulkano;
use vulkano::buffer::{BufferAccess, BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::device::{Device, Queue};
use vulkano::framebuffer::{Framebuffer, Subpass};
use vulkano::image::SwapchainImage;
use vulkano::instance::Instance;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::swapchain;
use vulkano::swapchain::{
    AcquireError, PresentMode, SurfaceTransform, Swapchain, SwapchainCreationError,
};
use vulkano::sync::{now, GpuFuture};
use vulkano_win;
use vulkano_win::VkSurfaceBuild;
use winit;

use vulkano::framebuffer::{
    AttachmentsList, EmptySinglePassRenderPassDesc, RenderPass, RenderPassAbstract,
};

#[derive(Debug, Clone)]
struct Vertex {
    position: [f32; 2],
}
impl_vertex!(Vertex, position);

pub struct VulkanRenderer {
    events_loop: winit::EventsLoop,
    window: vulkano_win::Window,
    width: u32,
    height: u32,
    device: Arc<Device>,
    queue: Arc<Queue>,
    recreate_swapchain: bool,
    previous_frame_end: Box<GpuFuture>,
    swapchain: Arc<Swapchain>,
    images: Vec<Arc<SwapchainImage>>,
    render_pass: Arc<RenderPassAbstract + Send + Sync>,
    pipeline: Arc<GraphicsPipelineAbstract + Send + Sync>,
    framebuffers: Option<
        Vec<Arc<Framebuffer<Arc<RenderPassAbstract + Send + Sync>, ((), Arc<SwapchainImage>)>>>,
    >,
    vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    num_frames_rendered: u64,
}

impl VulkanRenderer {
    pub fn new(app_settings: &ApplicationSettings) -> Result<Self, String> {
        // NOTE: match is used instead of ? because of mismatched types;
        // unwrap_or doesn't work due to the type inferencing not being
        // able to figure out what the correct value type is.
        // redo this later possibly with `chain_err`
        let instance: Arc<Instance> = {
            let extensions = vulkano_win::required_extensions();

            match Instance::new(None, &extensions, None) {
                Ok(i) => i,
                Err(_) => return Err("Could not create a Vulkan instance".to_owned()),
            }
        };

        let physical = match vulkano::instance::PhysicalDevice::enumerate(&instance).next() {
            Some(pd) => pd,
            None => return Err("No device available for Vulkan".to_owned()),
        };
        debug!(
            "Using device: {} (type: {:?})",
            physical.name(),
            physical.ty()
        );

        let events_loop = winit::EventsLoop::new();
        let window =
            match winit::WindowBuilder::new().build_vk_surface(&events_loop, instance.clone()) {
                Ok(w) => w,
                Err(_) => return Err("Could not create winit window for Vulkan".to_owned()),
            };

        let dimensions = {
            let (width, height) = window.window().get_inner_size_pixels().unwrap();
            [width, height]
        };

        //TODO: transfer queue and graphics queue
        let queue = physical
            .queue_families()
            .find(|&q| {
                // We take the first queue that supports drawing to our window.
                q.supports_graphics() && window.surface().is_supported(q).unwrap_or(false)
            })
            .expect("couldn't find a graphical queue family");

        let (device, mut queues) = {
            let device_ext = vulkano::device::DeviceExtensions {
                khr_swapchain: true,
                ..vulkano::device::DeviceExtensions::none()
            };

            Device::new(
                physical,
                physical.supported_features(),
                &device_ext,
                [(queue, 0.5)].iter().cloned(),
            )
            .expect("failed to create device")
        };

        let queue = queues.next().unwrap();

        let (swapchain, images) = {
            // Querying the capabilities of the surface. When we create the swapchain we can only
            // pass values that are allowed by the capabilities.
            let caps = window
                .surface()
                .capabilities(physical)
                .expect("failed to get surface capabilities");

            let alpha = caps.supported_composite_alpha.iter().next().unwrap();
            let format = caps.supported_formats[0].0;

            // Please take a look at the docs for the meaning of the parameters we didn't mention.
            Swapchain::new(
                device.clone(),
                window.surface().clone(),
                caps.min_image_count,
                format,
                dimensions,
                1,
                caps.supported_usage_flags,
                &queue,
                SurfaceTransform::Identity,
                alpha,
                PresentMode::Fifo,
                true,
                None,
            )
            .expect("failed to create swapchain")
        };

        // We now create a buffer that will store the shape of our triangle.
        let vertex_buffer = {
            CpuAccessibleBuffer::from_iter(
                device.clone(),
                BufferUsage::all(),
                [
                    Vertex {
                        position: [-0.5, -0.25],
                    },
                    Vertex {
                        position: [0.0, 0.5],
                    },
                    Vertex {
                        position: [0.25, -0.1],
                    },
                ]
                .iter()
                .cloned(),
            )
            .expect("failed to create buffer")
        };

        mod vs {
            #[derive(VulkanoShader)]
            #[ty = "vertex"]
            #[src = "
#version 450
layout(location = 0) in vec2 position;
void main() {
    gl_Position = vec4(position, 0.0, 1.0);
}
"]
            struct Dummy;
        }

        mod fs {
            #[derive(VulkanoShader)]
            #[ty = "fragment"]
            #[src = "
#version 450
layout(location = 0) out vec4 f_color;
void main() {
    f_color = vec4(1.0, 0.0, 0.0, 1.0);
}
"]
            struct Dummy;
        }

        let vs = vs::Shader::load(device.clone()).expect("failed to create shader module");
        let fs = fs::Shader::load(device.clone()).expect("failed to create shader module");

        let render_pass = Arc::new(
            single_pass_renderpass!(device.clone(),
                    attachments: {
                        // `color` is a custom name we give to the first and only attachment.
                        color: {
                            // `load: Clear` means that we ask the GPU to clear the content of this
                            // attachment at the start of the drawing.
                            load: Clear,
                            // `store: Store` means that we ask the GPU to store the output of the draw
                            // in the actual image. We could also ask it to discard the result.
                            store: Store,
                            // `format: <ty>` indicates the type of the format of the image. This has to
                            // be one of the types of the `vulkano::format` module (or alternatively one
                            // of your structs that implements the `FormatDesc` trait). Here we use the
                            // generic `vulkano::format::Format` enum because we don't know the format in
                            // advance.
                            format: swapchain.format(),
                            // TODO:
                            samples: 1,
                        }
                    },
                    pass: {
                        // We use the attachment named `color` as the one and only color attachment.
                        color: [color],
                        // No depth-stencil attachment is indicated with empty brackets.
                        depth_stencil: {}
                    }
            )
            .unwrap(),
        );

        let pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<Vertex>()
                .vertex_shader(vs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fs.main_entry_point(), ())
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(device.clone())
                .unwrap(),
        );

        let mut framebuffers: Option<Vec<Arc<vulkano::framebuffer::Framebuffer<_, _>>>> = None;

        let mut previous_frame_end = Box::new(now(device.clone())) as Box<GpuFuture>;
        Ok(VulkanRenderer {
            events_loop,
            window,
            width: dimensions[0],
            height: dimensions[1],
            device,
            queue,
            recreate_swapchain: false,
            previous_frame_end,
            swapchain,
            images,
            render_pass,
            pipeline,
            framebuffers,
            vertex_buffer,
            num_frames_rendered: 0,
        })
    }
}

impl Renderer for VulkanRenderer {
    fn draw_gameboy(&mut self, gameboy: &Cpu, app_settings: &ApplicationSettings) {
        self.previous_frame_end.cleanup_finished();
        loop {
            if self.recreate_swapchain {
                let (width, height) = self.window.window().get_inner_size_pixels().unwrap();
                self.width = width;
                self.height = height;

                let dimensions = [width, height];

                let (new_swapchain, new_images) =
                    match self.swapchain.recreate_with_dimension(dimensions) {
                        Ok(r) => r,
                        // This error tends to happen when the user is manually resizing the window.
                        // Simply restarting the loop is the easiest way to fix this issue.
                        Err(SwapchainCreationError::UnsupportedDimensions) => {
                            continue;
                        }
                        Err(err) => panic!("{:?}", err),
                    };

                mem::replace(&mut self.swapchain, new_swapchain);
                mem::replace(&mut self.images, new_images);

                self.framebuffers = None;

                self.recreate_swapchain = false;
            }

            if self.framebuffers.is_none() {
                let new_framebuffers = Some(
                    self.images
                        .iter()
                        .map(|image| {
                            Arc::new(
                                Framebuffer::start(self.render_pass.clone())
                                    .add(image.clone())
                                    .unwrap()
                                    .build()
                                    .unwrap(),
                            )
                        })
                        .collect::<Vec<_>>(),
                );
                mem::replace(&mut self.framebuffers, new_framebuffers);
            }

            let (image_num, acquire_future) =
                match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                    Ok(r) => r,
                    Err(AcquireError::OutOfDate) => {
                        self.recreate_swapchain = true;
                        continue;
                    }
                    Err(err) => panic!("{:?}", err),
                };

            let dimensions = [self.width, self.height];

            let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(
                self.device.clone(),
                self.queue.family(),
            )
            .unwrap()
            // Before we can draw, we have to *enter a render pass*. There are two methods to do
            // this: `draw_inline` and `draw_secondary`. The latter is a bit more advanced and is
            // not covered here.
            //
            // The third parameter builds the list of values to clear the attachments with. The API
            // is similar to the list of attachments when building the framebuffers, except that
            // only the attachments that use `load: Clear` appear in the list.
            .begin_render_pass(
                self.framebuffers.as_ref().unwrap()[image_num].clone(),
                false,
                vec![[0.0, 0.0, 1.0, 1.0].into()],
            )
            .unwrap()
            // We are now inside the first subpass of the render pass. We add a draw command.
            //
            // The last two parameters contain the list of resources to pass to the shaders.
            // Since we used an `EmptyPipeline` object, the objects have to be `()`.
            .draw(
                self.pipeline.clone(),
                DynamicState {
                    line_width: None,
                    // TODO: Find a way to do this without having to
                    //dynamically allocate a Vec every frame.
                    viewports: Some(vec![Viewport {
                        origin: [0.0, 0.0],
                        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                        depth_range: 0.0..1.0,
                    }]),
                    scissors: None,
                },
                vec![self.vertex_buffer.clone()],
                (),
                (),
            )
            .unwrap()
            // We leave the render pass by calling `draw_end`. Note that if we had multiple
            // subpasses we could have called `next_inline` (or `next_secondary`) to jump to the
            // next subpass.
            .end_render_pass()
            .unwrap()
            // Finish building the command buffer by calling `build`.
            .build()
            .unwrap();

            let mut prev_frame_end: Box<GpuFuture> = Box::new(now(self.device.clone()));

            mem::swap(&mut self.previous_frame_end, &mut prev_frame_end);
            let future = prev_frame_end
                .join(acquire_future)
                .then_execute(self.queue.clone(), command_buffer)
                .unwrap()
                .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_num)
                .then_signal_fence_and_flush()
                .unwrap();

            self.previous_frame_end = Box::new(future) as Box<_>;
            self.num_frames_rendered += 1;
            break;
        }
    }

    fn draw_memory_visualization(&mut self, gameboy: &Cpu, app_settings: &ApplicationSettings) {
        unimplemented!();
    }

    fn handle_events(
        &mut self,
        gameboy: &mut Cpu,
        app_settings: &ApplicationSettings,
    ) -> Vec<renderer::EventResponse> {
        let mut ret_vec = vec![];

        if self.num_frames_rendered % 100 == 0 {
            debug!("{} frames rendered", self.num_frames_rendered);
        }
        self.events_loop.poll_events(|ev| match ev {
            winit::Event::WindowEvent {
                event: winit::WindowEvent::Closed,
                ..
            } => ret_vec.push(EventResponse::ProgramTerminated),
            _ => (),
        });
        return ret_vec;
    }
}
