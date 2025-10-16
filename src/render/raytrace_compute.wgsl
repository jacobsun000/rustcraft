struct RayUniforms {
    inv_view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    grid_origin: vec4<i32>,
    grid_size: vec4<u32>,
    stride: vec4<u32>,
    atlas: vec4<u32>,
};

@group(0) @binding(0)
var target_image: texture_storage_2d<rgba8unorm, write>;

@group(0) @binding(1)
var<uniform> uniforms: RayUniforms;

@group(0) @binding(2)
var<storage, read> voxels: array<u32>;

struct BlockInfo {
    face_tiles: array<u32, 6>,
    luminance: f32,
    reflectivity: f32,
    _padding: vec2<f32>,
};

@group(0) @binding(3)
var<storage, read> block_data: array<BlockInfo>;

@group(0) @binding(4)
var block_atlas: texture_2d<f32>;

@group(0) @binding(5)
var atlas_sampler: sampler;

const SUN_DIRECTION: vec3<f32> = vec3<f32>(0.3, 0.9, 0.5);

fn lerp_vec3(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    return a + t * (b - a);
}

fn decode_tile(tile: u32) -> vec2<u32> {
    let x = tile & 0xFFFFu;
    let y = tile >> 16u;
    return vec2<u32>(x, y);
}

fn atlas_coords(tile: u32, uv: vec2<f32>) -> vec2<f32> {
    let coords = decode_tile(tile);
    let tile_size = f32(uniforms.atlas.x);
    let atlas_width = f32(uniforms.atlas.y);
    let atlas_height = f32(uniforms.atlas.z);
    let pixel = vec2<f32>(
        f32(coords.x) * tile_size + uv.x * (tile_size - 1.0) + 0.5,
        f32(coords.y) * tile_size + uv.y * (tile_size - 1.0) + 0.5,
    );
    return vec2<f32>(pixel.x / atlas_width, pixel.y / atlas_height);
}

fn sample_tile(tile: u32, uv: vec2<f32>) -> vec3<f32> {
    let coords = atlas_coords(tile, uv);
    return textureSampleLevel(block_atlas, atlas_sampler, coords, 0.0).rgb;
}

fn face_index(normal: vec3<f32>) -> u32 {
    if normal.x < -0.5 {
        return 0u;
    }
    if normal.x > 0.5 {
        return 1u;
    }
    if normal.y < -0.5 {
        return 2u;
    }
    if normal.y > 0.5 {
        return 3u;
    }
    if normal.z < -0.5 {
        return 4u;
    }
    return 5u;
}

fn face_uv(normal: vec3<f32>, local: vec3<f32>) -> vec2<f32> {
    let clamped = clamp(local, vec3<f32>(0.0), vec3<f32>(0.999));
    if normal.x > 0.5 {
        return vec2<f32>(clamped.z, clamped.y);
    }
    if normal.x < -0.5 {
        return vec2<f32>(1.0 - clamped.z, clamped.y);
    }
    if normal.y > 0.5 {
        return vec2<f32>(clamped.x, clamped.z);
    }
    if normal.y < -0.5 {
        return vec2<f32>(clamped.x, 1.0 - clamped.z);
    }
    if normal.z > 0.5 {
        return vec2<f32>(clamped.x, clamped.y);
    }
    return vec2<f32>(clamped.x, clamped.y);
}

fn voxel_count() -> u32 {
    return uniforms.stride.y * uniforms.grid_size.z;
}

fn voxel_index(coord: vec3<i32>) -> u32 {
    let origin = uniforms.grid_origin.xyz;
    let local = coord - origin;
    if any(local < vec3<i32>(0)) {
        return voxel_count();
    }
    let size = uniforms.grid_size.xyz;
    let lx = u32(local.x);
    let ly = u32(local.y);
    let lz = u32(local.z);
    if lx >= size.x || ly >= size.y || lz >= size.z {
        return voxel_count();
    }
    let stride_y = uniforms.stride.x;
    let stride_z = uniforms.stride.y;
    return lx + ly * stride_y + lz * stride_z;
}

fn sample_block(coord: vec3<i32>) -> u32 {
    let idx = voxel_index(coord);
    if idx >= voxel_count() {
        return 0u;
    }
    return voxels[idx];
}

fn intersect_aabb(origin: vec3<f32>, dir: vec3<f32>, min: vec3<f32>, max: vec3<f32>) -> vec2<f32> {
    var t_min = -1e30;
    var t_max = 1e30;

    let dx = dir.x;
    if abs(dx) < 1e-5 {
        if origin.x < min.x || origin.x > max.x {
            return vec2<f32>(1.0, -1.0);
        }
    } else {
        var tx0 = (min.x - origin.x) / dx;
        var tx1 = (max.x - origin.x) / dx;
        if tx0 > tx1 {
            let temp = tx0;
            tx0 = tx1;
            tx1 = temp;
        }
        t_min = max(t_min, tx0);
        t_max = min(t_max, tx1);
        if t_max < t_min {
            return vec2<f32>(1.0, -1.0);
        }
    }

    let dy = dir.y;
    if abs(dy) < 1e-5 {
        if origin.y < min.y || origin.y > max.y {
            return vec2<f32>(1.0, -1.0);
        }
    } else {
        var ty0 = (min.y - origin.y) / dy;
        var ty1 = (max.y - origin.y) / dy;
        if ty0 > ty1 {
            let temp = ty0;
            ty0 = ty1;
            ty1 = temp;
        }
        t_min = max(t_min, ty0);
        t_max = min(t_max, ty1);
        if t_max < t_min {
            return vec2<f32>(1.0, -1.0);
        }
    }

    let dz = dir.z;
    if abs(dz) < 1e-5 {
        if origin.z < min.z || origin.z > max.z {
            return vec2<f32>(1.0, -1.0);
        }
    } else {
        var tz0 = (min.z - origin.z) / dz;
        var tz1 = (max.z - origin.z) / dz;
        if tz0 > tz1 {
            let temp = tz0;
            tz0 = tz1;
            tz1 = temp;
        }
        t_min = max(t_min, tz0);
        t_max = min(t_max, tz1);
        if t_max < t_min {
            return vec2<f32>(1.0, -1.0);
        }
    }

    return vec2<f32>(t_min, t_max);
}

fn determine_entry_normal(pos: vec3<f32>, min: vec3<f32>, max: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    let eps = 1e-3;
    if abs(pos.x - min.x) < eps {
        return vec3<f32>(-1.0, 0.0, 0.0);
    }
    if abs(max.x - pos.x) < eps {
        return vec3<f32>(1.0, 0.0, 0.0);
    }
    if abs(pos.y - min.y) < eps {
        return vec3<f32>(0.0, -1.0, 0.0);
    }
    if abs(max.y - pos.y) < eps {
        return vec3<f32>(0.0, 1.0, 0.0);
    }
    if abs(pos.z - min.z) < eps {
        return vec3<f32>(0.0, 0.0, -1.0);
    }
    if abs(max.z - pos.z) < eps {
        return vec3<f32>(0.0, 0.0, 1.0);
    }

    let ad = abs(dir);
    if ad.x >= ad.y && ad.x >= ad.z {
        return vec3<f32>(-sign(dir.x), 0.0, 0.0);
    }
    if ad.y >= ad.x && ad.y >= ad.z {
        return vec3<f32>(0.0, -sign(dir.y), 0.0);
    }
    return vec3<f32>(0.0, 0.0, -sign(dir.z));
}

fn compute_t_max(origin: f32, direction: f32, voxel: i32, step: i32) -> f32 {
    if step == 0 {
        return 1e30;
    }
    var boundary = f32(voxel);
    if step > 0 {
        boundary = f32(voxel + 1);
    }
    return (boundary - origin) / direction;
}

fn compute_step_delta(direction: f32, step: i32) -> f32 {
    if step == 0 {
        return 1e30;
    }
    return abs(1.0 / direction);
}

fn sky(dir: vec3<f32>) -> vec3<f32> {
    let t = clamp(dir.y * 0.5 + 0.5, 0.0, 1.0);
    let horizon = vec3<f32>(0.4, 0.5, 0.7);
    let zenith = vec3<f32>(0.05, 0.09, 0.15);
    return lerp_vec3(horizon, zenith, t);
}

struct HitResult {
    block: u32,
    voxel: vec3<i32>,
    normal: vec3<f32>,
    travel: f32,
}

struct SurfaceSample {
    color: vec3<f32>,
    fog_color: vec3<f32>,
    fog: f32,
    reflectivity: f32,
    reflection_origin: vec3<f32>,
    reflection_dir: vec3<f32>,
    has_reflection: bool,
}

fn miss_hit() -> HitResult {
    return HitResult(0u, vec3<i32>(0, 0, 0), vec3<f32>(0.0, 0.0, 0.0), 0.0);
}

fn trace_ray(origin: vec3<f32>, dir: vec3<f32>) -> HitResult {
    let grid_origin_i = uniforms.grid_origin.xyz;
    let grid_min = vec3<f32>(
        f32(grid_origin_i.x),
        f32(grid_origin_i.y),
        f32(grid_origin_i.z),
    );
    let grid_size_u = uniforms.grid_size.xyz;
    let grid_extent = vec3<f32>(
        f32(grid_size_u.x),
        f32(grid_size_u.y),
        f32(grid_size_u.z),
    );
    let grid_max = grid_min + grid_extent;

    let bounds = intersect_aabb(origin, dir, grid_min, grid_max);
    if bounds.x > bounds.y {
        return miss_hit();
    }

    var entry = max(bounds.x, 0.0);
    let exit = bounds.y;
    if exit <= entry {
        return miss_hit();
    }

    let start = origin + dir * (entry + 1e-3);
    let start_floor = floor(start);
    var voxel = vec3<i32>(
        i32(start_floor.x),
        i32(start_floor.y),
        i32(start_floor.z),
    );

    var step_vec = vec3<i32>(0, 0, 0);
    if dir.x > 0.0 {
        step_vec.x = 1;
    } else if dir.x < 0.0 {
        step_vec.x = -1;
    }
    if dir.y > 0.0 {
        step_vec.y = 1;
    } else if dir.y < 0.0 {
        step_vec.y = -1;
    }
    if dir.z > 0.0 {
        step_vec.z = 1;
    } else if dir.z < 0.0 {
        step_vec.z = -1;
    }

    var t_max = vec3<f32>(
        compute_t_max(start.x, dir.x, voxel.x, step_vec.x),
        compute_t_max(start.y, dir.y, voxel.y, step_vec.y),
        compute_t_max(start.z, dir.z, voxel.z, step_vec.z),
    );
    let delta = vec3<f32>(
        compute_step_delta(dir.x, step_vec.x),
        compute_step_delta(dir.y, step_vec.y),
        compute_step_delta(dir.z, step_vec.z),
    );

    var normal = determine_entry_normal(start, grid_min, grid_max, dir);
    var block = sample_block(voxel);
    if block != 0u {
        return HitResult(block, voxel, normal, entry);
    }

    var travel = entry;
    let max_steps = (uniforms.grid_size.x + uniforms.grid_size.y + uniforms.grid_size.z) * 4u;
    var steps: u32 = 0u;

    loop {
        if steps >= max_steps {
            break;
        }

        var axis: u32 = 0u;
        if t_max.x < t_max.y {
            if t_max.x < t_max.z {
                axis = 0u;
            } else {
                axis = 2u;
            }
        } else {
            if t_max.y < t_max.z {
                axis = 1u;
            } else {
                axis = 2u;
            }
        }

        if axis == 0u {
            voxel.x += step_vec.x;
            travel = t_max.x;
            t_max.x += delta.x;
            normal = vec3<f32>(-f32(step_vec.x), 0.0, 0.0);
        } else if axis == 1u {
            voxel.y += step_vec.y;
            travel = t_max.y;
            t_max.y += delta.y;
            normal = vec3<f32>(0.0, -f32(step_vec.y), 0.0);
        } else {
            voxel.z += step_vec.z;
            travel = t_max.z;
            t_max.z += delta.z;
            normal = vec3<f32>(0.0, 0.0, -f32(step_vec.z));
        }

        if travel > exit {
            break;
        }

        block = sample_block(voxel);
        if block != 0u {
            return HitResult(block, voxel, normal, travel);
        }

        steps = steps + 1u;
    }

    return miss_hit();
}

fn evaluate_surface(hit: HitResult, origin: vec3<f32>, dir: vec3<f32>) -> SurfaceSample {
    let info = block_data[hit.block];
    let hit_point = origin + dir * (hit.travel + 1e-4);
    let block_origin = vec3<f32>(
        f32(hit.voxel.x),
        f32(hit.voxel.y),
        f32(hit.voxel.z),
    );
    let local = hit_point - block_origin;
    let face = face_index(hit.normal);
    var tile = info.face_tiles[0u];
    switch face {
        case 0u: {
            tile = info.face_tiles[0u];
        }
        case 1u: {
            tile = info.face_tiles[1u];
        }
        case 2u: {
            tile = info.face_tiles[2u];
        }
        case 3u: {
            tile = info.face_tiles[3u];
        }
        case 4u: {
            tile = info.face_tiles[4u];
        }
        default: {
            tile = info.face_tiles[5u];
        }
    }
    let uv = face_uv(hit.normal, local);
    let albedo = sample_tile(tile, uv);

    let sun = normalize(SUN_DIRECTION);
    let light = max(dot(hit.normal, sun), 0.0);
    let diffuse = 0.25 + 0.75 * light;
    let emission = info.luminance;
    let brightness = diffuse + emission * 0.08;
    let surface = albedo * brightness;
    let glow = albedo * emission * 0.12;
    let fog_color = vec3<f32>(0.6, 0.75, 0.95);
    let fog = clamp(hit.travel / 400.0, 0.0, 1.0) * 0.6;

    let reflectivity = clamp(info.reflectivity, 0.0, 1.0);
    var reflection_origin = vec3<f32>(0.0, 0.0, 0.0);
    var reflection_dir = vec3<f32>(0.0, 0.0, 0.0);
    var has_reflection = false;
    if reflectivity > 0.001 {
        var candidate = reflect(dir, hit.normal);
        let dir_len = length(candidate);
        if dir_len > 1e-5 {
            reflection_dir = candidate / dir_len;
            reflection_origin = hit_point + hit.normal * 1e-3;
            has_reflection = true;
        }
    }

    return SurfaceSample(
        surface + glow,
        fog_color,
        fog,
        reflectivity,
        reflection_origin,
        reflection_dir,
        has_reflection,
    );
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let resolution = uniforms.stride.zw;
    if gid.x >= resolution.x || gid.y >= resolution.y {
        return;
    }

    let res = vec2<f32>(f32(resolution.x), f32(resolution.y));
    let pixel = vec2<f32>(f32(gid.x) + 0.5, f32(gid.y) + 0.5);
    let ndc = vec2<f32>(
        pixel.x / res.x * 2.0 - 1.0,
        1.0 - pixel.y / res.y * 2.0,
    );

    let clip = vec4<f32>(ndc, 1.0, 1.0);
    let world = uniforms.inv_view_proj * clip;
    let world_pos = world.xyz / world.w;
    let origin = uniforms.eye.xyz;
    let dir = normalize(world_pos - origin);

    let hit = trace_ray(origin, dir);
    var color = sky(dir);
    if hit.block != 0u {
        let sample = evaluate_surface(hit, origin, dir);
        var shaded = sample.color;

        if sample.reflectivity > 0.001 && sample.has_reflection {
            var reflection_color = sky(sample.reflection_dir);
            let reflection_hit = trace_ray(sample.reflection_origin, sample.reflection_dir);
            if reflection_hit.block != 0u {
                let reflection_sample =
                    evaluate_surface(reflection_hit, sample.reflection_origin, sample.reflection_dir);
                reflection_color =
                    lerp_vec3(reflection_sample.color, reflection_sample.fog_color, reflection_sample.fog);
            }
            shaded = shaded * (1.0 - sample.reflectivity) + reflection_color * sample.reflectivity;
        }

        color = lerp_vec3(shaded, sample.fog_color, sample.fog);
    }

    textureStore(target_image, vec2<i32>(gid.xy), vec4<f32>(color, 1.0));
}
