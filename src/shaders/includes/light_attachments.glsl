// The `color_input` parameter of the `draw` method.
layout(input_attachment_index = 0, set = 1, binding = 0) uniform subpassInput u_diffuse;
// The `normals_input` parameter of the `draw` method.
layout(input_attachment_index = 1, set = 1, binding = 1) uniform subpassInput u_normals;
// The `depth_input` parameter of the `draw` method.
layout(input_attachment_index = 2, set = 1, binding = 2) uniform subpassInput u_depth;