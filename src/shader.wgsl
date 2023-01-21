// Vertex shader
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> ratios: vec2<f32>;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = in.tex_coords;
    out.clip_position = vec4<f32>(in.position.x * ratios.x, in.position.y * ratios.y, 0.0, 1.0);
    return out;
}

@group(1) @binding(0)
var screen_texture: texture_2d<f32>;
@group(1) @binding(1)
var screen_texture_sampler: sampler;

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(screen_texture, screen_texture_sampler, in.tex_coords);
}
