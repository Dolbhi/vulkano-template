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
- [x] Multiple mesh rendering
- [x] Multiple pipeline rendering
- [x] Realtime object rendering
	- [ ] Render object streaming
- [x] .obj loading
- [x] Global data dynamic buffer
- [x] Object data storage buffer
- [x] Depth culling
- [ ] Lighting (Deferred rendering)
	- [ ] Multiple subpasses?
- [ ] GUI
- [x] Camera control
- [ ] View aspect adjust with window aspect
- [ ] Textures
- [ ] Dynamic meshes
- [ ] Anti-aliasing
- [ ] Don't clean every frame

### Issues
- [x] Crashes when minimised
- [ ] Crashes when closed after resize?
