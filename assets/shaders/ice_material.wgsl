#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::forward_io::VertexOutput

@group(2) @binding(0) var<uniform> base_color: vec4<f32>;
@group(2) @binding(1) var reflection_texture: texture_2d<f32>;
@group(2) @binding(2) var reflection_sampler: sampler;

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    // Calculate screen space coordinates (0.0 to 1.0)
    // view.viewport is vec4<f32>(x, y, width, height)
    let resolution = vec2<f32>(view.viewport.z, view.viewport.w);
    let screen_uv = mesh.position.xy / resolution;

    // Sample the reflection texture
    let reflection = textureSample(reflection_texture, reflection_sampler, screen_uv);

    // Mix base color with reflection
    // Assuming base_color is the ice color (semi-transparent)
    // and reflection adds to it.

    // Simple additive blending for reflection
    let final_color = base_color + reflection * 0.7; // 0.7 intensity

    return final_color;
}
