@group(0) @binding(0)
var font_texture: texture_2d<f32>;

@group(0) @binding(1)
var font_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = uv;
    out.color = color;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(font_texture, font_sampler, input.uv);
    let alpha = sample.a * input.color.a;
    if (alpha <= 0.0) {
        discard;
    }
    return vec4<f32>(input.color.rgb * sample.a, alpha);
}
