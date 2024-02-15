#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var fb_depth: texture_depth_2d;

@fragment
fn main_fs(in: FullscreenVertexOutput) -> @builtin(frag_depth) f32 {
    let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
    let index = uv * vec2<f32>(textureDimensions(fb_depth));

    let fb_depth = textureLoad(fb_depth, vec2<i32>(index), 0);
    return 1 - fb_depth;
}
