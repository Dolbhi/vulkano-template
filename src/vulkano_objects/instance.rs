//! Single function for creating instance, with required extensions from event loop
//! Reusable for multiple renderers

use std::sync::Arc;

use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo, LayerProperties};

const LIST_AVAILABLE_LAYERS: bool = false;
const ENABLE_VALIDATION_LAYERS: bool = false;
const VALIDATION_LAYERS: &[&str] = &["VK_LAYER_LUNARG_api_dump"];

/// Creates an instance with the required extensions from the given EventLoop, set to use no layers
pub fn get_instance(event_loop: &winit::event_loop::EventLoop<()>) -> Arc<Instance> {
    let library = vulkano::VulkanLibrary::new().expect("no local Vulkan library/DLL");
    let required_extensions = vulkano::swapchain::Surface::required_extensions(event_loop); // vulkano_win::required_extensions(&library);

    if LIST_AVAILABLE_LAYERS {
        let layers: Vec<_> = library.layer_properties().unwrap().collect();
        let layer_names = layers.iter().map(LayerProperties::name);
        println!(
            "Available layers:\n {:?}",
            layer_names.clone().collect::<Vec<&str>>()
        );
    }

    let mut create_info = InstanceCreateInfo {
        flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
        enabled_extensions: required_extensions,
        ..Default::default()
    };

    if ENABLE_VALIDATION_LAYERS {
        create_info.enabled_layers = VALIDATION_LAYERS.iter().map(|s| s.to_string()).collect();
    }

    Instance::new(library, create_info).unwrap()
}
