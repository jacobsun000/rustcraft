struct RayUniforms {
    frustum: array<vec4<f32>, 4>,
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
    specular: f32,
    diffuse: f32,
    roughness: f32,
    metallic: f32,
    transmission: f32,
    ior: f32,
    transmission_tint: f32,
};

@group(0) @binding(3)
var<storage, read> block_data: array<BlockInfo>;

@group(0) @binding(4)
var block_atlas: texture_2d<f32>;

@group(0) @binding(5)
var atlas_sampler: sampler;

const SUN_DIRECTION: vec3<f32> = vec3<f32>(0.2795085, 0.8385254, 0.4658469);
const PI: f32 = 3.14159265359;
const MAX_SPECULAR_BOUNCES: u32 = 2u;
const ROUGH_SPECULAR_LIMIT: f32 = 0.4;
const DIFFUSE_SAMPLE_WEIGHT: f32 = 0.6;
const MAX_TRANSMISSION_BOUNCES: u32 = 2u;

fn lerp_vec3(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    return a + t * (b - a);
}

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
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

fn schlick(f0: f32, cos_theta: f32) -> f32 {
    let base = saturate(1.0 - cos_theta);
    let factor = base * base * base * base * base;
    return f0 + (1.0 - f0) * factor;
}

fn hash_u32(value: u32) -> u32 {
    var x = value;
    x = (x ^ (x >> 17u)) * 0xed5ad4bbu;
    x = (x ^ (x >> 11u)) * 0xac4c1b51u;
    x = (x ^ (x >> 15u)) * 0x31848babu;
    return x ^ (x >> 14u);
}

fn scramble_seed(seed: vec3<u32>, offset: u32) -> u32 {
    let mix = seed.x ^ (seed.y * 0x9e3779b9u) ^ (seed.z * 0x7f4a7c15u) ^ offset;
    return hash_u32(mix);
}

fn random_scalar(seed: vec3<u32>, offset: u32) -> f32 {
    let value = scramble_seed(seed, offset);
    let mantissa = value & 0x00FFFFFFu;
    return f32(mantissa) / f32(0x01000000u);
}

fn random_vec2(seed: vec3<u32>, offset: u32) -> vec2<f32> {
    return vec2<f32>(
        random_scalar(seed, offset),
        random_scalar(seed, offset ^ 0xa511e9b3u),
    );
}

fn orthonormal_basis(normal: vec3<f32>) -> mat3x3<f32> {
    let up = select(
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(1.0, 0.0, 0.0),
        abs(normal.y) > 0.99,
    );
    let tangent = normalize(cross(up, normal));
    let bitangent = cross(normal, tangent);
    return mat3x3<f32>(tangent, bitangent, normal);
}

fn sample_cosine_hemisphere(normal: vec3<f32>, xi: vec2<f32>) -> vec3<f32> {
    let phi = 2.0 * PI * xi.x;
    let r = sqrt(xi.y);
    let local = vec3<f32>(
        r * cos(phi),
        r * sin(phi),
        sqrt(max(0.0, 1.0 - xi.y)),
    );
    let basis = orthonormal_basis(normal);
    return normalize(basis * local);
}

fn face_index(normal: vec3<f32>) -> u32 {
    let threshold = 0.5;
    if normal.x < -threshold {
        return 0u;
    }
    if normal.x > threshold {
        return 1u;
    }
    if normal.y < -threshold {
        return 2u;
    }
    if normal.y > threshold {
        return 3u;
    }
    if normal.z < -threshold {
        return 4u;
    }
    return 5u;
}

fn face_uv(normal: vec3<f32>, local: vec3<f32>) -> vec2<f32> {
    let clamped = clamp(local, vec3<f32>(0.0), vec3<f32>(0.999));
    if normal.x > 0.5 {
        return vec2<f32>(clamped.z, 1.0 - clamped.y);
    }
    if normal.x < -0.5 {
        return vec2<f32>(1.0 - clamped.z, 1.0 - clamped.y);
    }
    if normal.y > 0.5 {
        return vec2<f32>(clamped.x, clamped.z);
    }
    if normal.y < -0.5 {
        return vec2<f32>(clamped.x, 1.0 - clamped.z);
    }
    if normal.z > 0.5 {
        return vec2<f32>(clamped.x, 1.0 - clamped.y);
    }
    return vec2<f32>(clamped.x, 1.0 - clamped.y);
}

fn tile_for_face(info: BlockInfo, face: u32) -> u32 {
    if face == 0u {
        return info.face_tiles[0u];
    }
    if face == 1u {
        return info.face_tiles[1u];
    }
    if face == 2u {
        return info.face_tiles[2u];
    }
    if face == 3u {
        return info.face_tiles[3u];
    }
    if face == 4u {
        return info.face_tiles[4u];
    }
    return info.face_tiles[5u];
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
    let word_index = idx >> 2u;
    let lane = (idx & 3u) * 8u;
    let packed = voxels[word_index];
    return (packed >> lane) & 0xFFu;
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

fn refract_snell(incident: vec3<f32>, normal: vec3<f32>, eta_i: f32, eta_t: f32) -> vec3<f32> {
    var n = normal;
    var cos_i = dot(incident, n);
    var eta = eta_i / eta_t;
    if cos_i > 0.0 {
        n = -n;
        cos_i = dot(incident, n);
    }
    cos_i = clamp(cos_i, -1.0, 1.0);
    let k = 1.0 - eta * eta * (1.0 - cos_i * cos_i);
    if k < 0.0 {
        return vec3<f32>(0.0);
    }
    return eta * incident - (eta * cos_i + sqrt(k)) * n;
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
    direct: vec3<f32>,
    specular: vec3<f32>,
    diffuse: vec3<f32>,
    transmission: vec3<f32>,
    fog_color: vec3<f32>,
    fog: f32,
}

struct MaterialInfo {
    position: vec3<f32>,
    normal: vec3<f32>,
    albedo: vec3<f32>,
    direct: vec3<f32>,
    specular: f32,
    diffuse: f32,
    roughness: f32,
    metallic: f32,
    transmission: f32,
    ior: f32,
    transmission_tint: f32,
    voxel: vec3<i32>,
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

fn gather_material(hit: HitResult, origin: vec3<f32>, dir: vec3<f32>) -> MaterialInfo {
    let info = block_data[hit.block];
    let hit_point = origin + dir * (hit.travel + 1e-4);
    let block_origin = vec3<f32>(
        f32(hit.voxel.x),
        f32(hit.voxel.y),
        f32(hit.voxel.z),
    );
    let local = hit_point - block_origin;
    let face = face_index(hit.normal);
    let tile = tile_for_face(info, face);
    let uv = face_uv(hit.normal, local);
    let albedo = sample_tile(tile, uv);

    let metallic = saturate(info.metallic);
    let transmission = saturate(info.transmission);
    let tint_mix = saturate(info.transmission_tint);
    let ior = max(info.ior, 1.0);

    let light = max(dot(hit.normal, SUN_DIRECTION), 0.0);
    let diffuse_base = albedo * light * saturate(info.diffuse);
    let diffuse_component = diffuse_base * (1.0 - metallic) * (1.0 - transmission);
    let emission = albedo * info.luminance * 0.12;
    let direct = diffuse_component + emission;
    let base_specular = saturate(info.specular);
    let specular = base_specular * (1.0 - metallic) + 0.95 * metallic;
    let diffuse_strength = saturate(info.diffuse) * (1.0 - metallic) * (1.0 - transmission);
    let roughness = clamp(info.roughness, 0.02, 1.0);

    return MaterialInfo(
        hit_point,
        hit.normal,
        albedo,
        direct,
        specular,
        diffuse_strength,
        roughness,
        metallic,
        transmission,
        ior,
        tint_mix,
        hit.voxel,
    );
}

fn trace_specular_chain(material: MaterialInfo, incoming: vec3<f32>, seed: vec3<u32>) -> vec3<f32> {
    let cos_theta = saturate(dot(material.normal, -incoming));
    let base_f0 = material.specular * (1.0 - material.metallic) + 0.96 * material.metallic;
    let base_reflect = schlick(base_f0, cos_theta);
    if base_reflect < 0.005 {
        return vec3<f32>(0.0);
    }

    var color = vec3<f32>(0.0);
    let base_tint = lerp_vec3(vec3<f32>(1.0), material.albedo, material.metallic);
    var throughput = base_tint * base_reflect * (1.0 - material.transmission);
    var ray_origin = material.position + material.normal * 1e-3;
    let jitter_seed = random_vec2(seed, 1u);
    let jitter = sample_cosine_hemisphere(material.normal, jitter_seed);
    var ray_dir = normalize(mix(reflect(incoming, material.normal), jitter, material.roughness));
    let allow_second = material.roughness < ROUGH_SPECULAR_LIMIT;
    let bounce_limit = select(1u, MAX_SPECULAR_BOUNCES, allow_second);

    for (var bounce = 0u; bounce < bounce_limit; bounce = bounce + 1u) {
        let hit = trace_ray(ray_origin, ray_dir);
        if hit.block == 0u {
            color += throughput * sky(ray_dir);
            break;
        }

        let sample_material = gather_material(hit, ray_origin, ray_dir);
        color += throughput * sample_material.direct;

        let next_cos = saturate(dot(sample_material.normal, -ray_dir));
        let sample_f0 =
            sample_material.specular * (1.0 - sample_material.metallic) + 0.96 * sample_material.metallic;
        let fresnel = schlick(sample_f0, next_cos);
        let tint = lerp_vec3(vec3<f32>(1.0), sample_material.albedo, sample_material.metallic);
        let attenuation = tint * fresnel * (1.0 - sample_material.roughness * 0.35) * (1.0 - sample_material.transmission * 0.5);

        throughput *= attenuation;
        if max(max(throughput.x, throughput.y), throughput.z) < 0.01 {
            break;
        }

        let xi = random_vec2(seed, 23u * (bounce + 1u) + 5u);
        let jitter_dir = sample_cosine_hemisphere(sample_material.normal, xi);
        ray_origin = sample_material.position + sample_material.normal * 1e-3;
        ray_dir =
            normalize(mix(reflect(ray_dir, sample_material.normal), jitter_dir, sample_material.roughness));
    }

    return color;
}

fn trace_diffuse_component(material: MaterialInfo, seed: vec3<u32>) -> vec3<f32> {
    if material.diffuse < 0.01 {
        return vec3<f32>(0.0);
    }

    let xi = random_vec2(seed, 11u);
    let bounce_dir = sample_cosine_hemisphere(material.normal, xi);
    let bounce_origin = material.position + material.normal * 1e-3;
    let hit = trace_ray(bounce_origin, bounce_dir);

    var indirect = material.albedo * material.diffuse * 0.1;
    if hit.block == 0u {
        indirect += material.albedo * material.diffuse * 0.25 * sky(bounce_dir);
    } else {
        let bounced = gather_material(hit, bounce_origin, bounce_dir);
        indirect += bounced.direct * material.diffuse * DIFFUSE_SAMPLE_WEIGHT;
    }

    return indirect;
}

fn trace_transmission(material: MaterialInfo, dir: vec3<f32>, seed: vec3<u32>) -> vec3<f32> {
    if material.transmission < 0.01 {
        return vec3<f32>(0.0);
    }

    let inside_dir = refract_snell(dir, material.normal, 1.0, material.ior);
    if length(inside_dir) < 1e-4 {
        return vec3<f32>(0.0);
    }

    let block_min = vec3<f32>(
        f32(material.voxel.x),
        f32(material.voxel.y),
        f32(material.voxel.z),
    );
    let block_max = block_min + vec3<f32>(1.0);
    let entry = material.position + inside_dir * 1e-4;
    let bounds = intersect_aabb(entry, inside_dir, block_min, block_max);
    if bounds.x > bounds.y {
        return vec3<f32>(0.0);
    }
    let exit_t = bounds.y;
    if exit_t <= 1e-4 {
        return vec3<f32>(0.0);
    }
    let exit_point = entry + inside_dir * (exit_t + 1e-4);
    let exit_normal = determine_entry_normal(exit_point, block_min, block_max, inside_dir);
    let exit_dir = refract_snell(inside_dir, exit_normal, material.ior, 1.0);
    if length(exit_dir) < 1e-4 {
        return vec3<f32>(0.0);
    }

    let tint = lerp_vec3(vec3<f32>(1.0), material.albedo, material.transmission_tint);
    let throughput = tint * material.transmission;
    let next_origin = exit_point + exit_dir * 1e-3;
    let next_hit = trace_ray(next_origin, exit_dir);
    if next_hit.block == 0u {
        return throughput * sky(exit_dir);
    }

    let bounced = gather_material(next_hit, next_origin, exit_dir);
    var color = throughput * bounced.direct;

    if bounced.diffuse > 0.02 && bounced.roughness > 0.12 {
        let diffuse_seed =
            vec3<u32>(seed.x ^ 0x6c8e9cf5u, seed.y + 0x52dce729u, seed.z + 0x7f4a7c15u);
        color += throughput * trace_diffuse_component(bounced, diffuse_seed);
    }

    let spec_seed = vec3<u32>(seed.x + 0x12345u, seed.y ^ 0x9e3779b9u, seed.z + 0x51ed1099u);
    color += throughput * trace_specular_chain(bounced, exit_dir, spec_seed);

    return color;
}

fn evaluate_surface(hit: HitResult, origin: vec3<f32>, dir: vec3<f32>, seed: vec3<u32>) -> SurfaceSample {
    let material = gather_material(hit, origin, dir);
    let specular = trace_specular_chain(material, dir, seed);
    var diffuse = vec3<f32>(0.0);
    if material.diffuse > 0.02 && material.roughness > 0.12 {
        diffuse = trace_diffuse_component(
            material,
            vec3<u32>(seed.x ^ 0x6c8e9cf5u, seed.y + 0x52dce729u, seed.z + 0x7f4a7c15u),
        );
    }
    let transmission = trace_transmission(
        material,
        dir,
        vec3<u32>(seed.x + 0xb5297a4du, seed.y ^ 0x68e31da4u, seed.z + 0x1b56c4f5u),
    );
    let fog_color = vec3<f32>(0.6, 0.75, 0.95);
    let fog = clamp(hit.travel / 400.0, 0.0, 1.0) * 0.6;

    return SurfaceSample(material.direct, specular, diffuse, transmission, fog_color, fog);
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let resolution = uniforms.stride.zw;
    if gid.x >= resolution.x || gid.y >= resolution.y {
        return;
    }

    let res = vec2<f32>(f32(resolution.x), f32(resolution.y));
    let pixel = vec2<f32>(f32(gid.x) + 0.5, f32(gid.y) + 0.5);
    let uv = pixel / res;

    let f0 = uniforms.frustum[0].xyz;
    let f1 = uniforms.frustum[1].xyz;
    let f2 = uniforms.frustum[2].xyz;
    let f3 = uniforms.frustum[3].xyz;

    let top = normalize(mix(f0, f1, uv.x));
    let bottom = normalize(mix(f2, f3, uv.x));
    let dir = normalize(mix(bottom, top, 1.0 - uv.y));
    let origin = uniforms.eye.xyz;
    let rng_seed = vec3<u32>(gid.x, gid.y, 0u);

    let hit = trace_ray(origin, dir);
    var color = sky(dir);
    if hit.block != 0u {
        let sample = evaluate_surface(hit, origin, dir, rng_seed);
        let shaded = sample.direct + sample.specular + sample.diffuse + sample.transmission;
        color = lerp_vec3(shaded, sample.fog_color, sample.fog);
    }

    textureStore(target_image, vec2<i32>(gid.xy), vec4<f32>(color, 1.0));
}
