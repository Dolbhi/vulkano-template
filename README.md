# Vulkano Renderer Template
A renderer using the [Vulkan API](https://www.khronos.org/vulkan/) written entirely in Rust, designed with the end goal of creating an entire game engine. 
### Main Resources Used
- [Vulkano](https://github.com/vulkano-rs/vulkano): A Rust wrapper around the Vulkan API, includes highly useful example scripts
- [Vulkano Book](https://github.com/vulkano-rs/vulkano-book/): This renderer is a fork of the tutorial source code
- [VulkanGuide](https://vkguide.dev/): Focused Vulkan guide on renderers for game engines

## To Do
### Misc.
- [ ] Combine render_loop and renderer

### Features
- [ ] Light data streaming
- [ ] Asset loader
	- [ ] Mesh, material and scene loading from files (with universal IDs)
	- [ ] Conversion into fast loading format
	- [ ] Loading objects/scenes from file (with auto loading of dependencies)
- [ ] Transparency
- [ ] GUI
- [ ] Dynamic meshes
- [ ] Anti-aliasing (MSAA)
- [ ] Occlusion culling
- [ ] Further lighting developments
	- [ ] Specular lighting
	- [ ] Ambient occlusion
	- [ ] Shadows
	- [ ] Multiple lighting materials
- [x] Multiple subpasses
- [x] Lighting (Deferred rendering)
- [x] Pipeline cache between materials
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
