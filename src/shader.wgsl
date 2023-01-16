// Vertex shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> ratios: vec2<u32>;

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // TODO: We really do a lot of calculations here, can this be avoided?

    var w: f32;
    var h: f32;
    if ratios.y * 2u > ratios.x {
        w = 1.0;
        h = f32(ratios.x) * 0.5 / f32(ratios.y);
    } else {
        w = f32(ratios.y) * 2.0 / f32(ratios.x);
        h = 1.0;
    }

    var x: f32;
    var y: f32;

    if in_vertex_index == 0u {
        x = -w;
        y = h;
    } else if in_vertex_index == 1u || in_vertex_index == 4u {
        x = -w;
        y = -h;
    } else if in_vertex_index == 2u || in_vertex_index == 3u {
        x = w;
        y = h;
    } else {
        x = w;
        y = -h;
    }

    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}
