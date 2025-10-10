struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u_camera: Camera;

@group(1) @binding(0)
var u_atlas: texture_2d<f32>;

@group(1) @binding(1)
var u_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = u_camera.view_proj * vec4<f32>(position, 1.0);
    out.color = color;
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(u_atlas, u_sampler, in.uv);
    let rgb = tex.rgb * in.color;
    return vec4<f32>(rgb, tex.a);
}
