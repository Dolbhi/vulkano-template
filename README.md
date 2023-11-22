# Vulkano Renderer Template
A renderer using the [Vulkan API](https://www.khronos.org/vulkan/) written entirely in Rust, designed with the end goal of creating an entire game engine. 
### Main Resources Used
- [Vulkano](https://github.com/vulkano-rs/vulkano): A Rust wrapper around the Vulkan API, includes highly useful example scripts
- [Vulkano Book](https://github.com/vulkano-rs/vulkano-book/): This renderer is a fork of the tutorial source code
- [VulkanGuide](https://vkguide.dev/): Focused Vulkan guide on renderers for game engines

## To Do
### Misc.
- [ ] Link to LinkedIn
	- [ ] Finish ReadMe
- [ ] Combine render_data::material with vulkano_objects::pipeline?
- [ ] Combine render_data::mesh with vulkano_objects::buffers?
- [ ] Refactor render_loop and renderer
	- [ ] Move resize logic to render_loop?

### Features
- [ ] Pipeline cache between materials
- [ ] Transparency
- [ ] Asset loader
- [ ] Lighting (Deferred rendering)
	- [ ] Multiple subpasses?
- [ ] Anti-aliasing (MSAA)
- [ ] GUI
- [ ] Dynamic meshes
- [ ] Don't clean every frame?
- [x] Multiple mesh rendering
- [x] Multiple pipeline rendering
- [x] Realtime dynamic object rendering
- [x] .obj loading
- [x] Global data dynamic buffer
- [x] Object data storage buffer
- [x] Depth culling
- [x] Camera control
- [x] View aspect adjust with window aspect
- [x] Textures

### Issues
- [x] Crashes when minimised
- [ ] Crashes when closed after resize?
