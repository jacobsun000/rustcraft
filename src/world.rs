use std::{collections::HashMap, f32::consts::PI};

pub const CHUNK_SIZE: usize = 16;
const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

#[derive(Clone, Copy)]
pub enum Block {
    Air,
    Solid([f32; 3]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

pub struct Chunk {
    blocks: Vec<Block>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            blocks: vec![Block::Air; CHUNK_VOLUME],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, block: Block) {
        let index = Self::index(x, y, z);
        self.blocks[index] = block;
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> Block {
        let index = Self::index(x, y, z);
        self.blocks[index]
    }

    fn index(x: usize, y: usize, z: usize) -> usize {
        x + CHUNK_SIZE * (z + CHUNK_SIZE * y)
    }
}

pub struct World {
    chunks: HashMap<ChunkCoord, Chunk>,
}

impl World {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    pub fn ensure_chunk(&mut self, coord: ChunkCoord) {
        self.chunks
            .entry(coord)
            .or_insert_with(|| generate_chunk(coord));
    }

    pub fn chunk(&self, coord: ChunkCoord) -> Option<&Chunk> {
        self.chunks.get(&coord)
    }

    pub fn block_at(&self, world_x: i32, world_y: i32, world_z: i32) -> Block {
        let chunk_coord = ChunkCoord {
            x: div_floor(world_x, CHUNK_SIZE as i32),
            y: div_floor(world_y, CHUNK_SIZE as i32),
            z: div_floor(world_z, CHUNK_SIZE as i32),
        };

        let local_x = mod_floor(world_x, CHUNK_SIZE as i32) as usize;
        let local_y = mod_floor(world_y, CHUNK_SIZE as i32) as usize;
        let local_z = mod_floor(world_z, CHUNK_SIZE as i32) as usize;

        self.chunk(chunk_coord)
            .map(|chunk| chunk.get(local_x, local_y, local_z))
            .unwrap_or(Block::Air)
    }
}

pub fn chunk_origin(coord: ChunkCoord) -> [f32; 3] {
    let half = CHUNK_SIZE as f32 / 2.0;
    [
        coord.x as f32 * CHUNK_SIZE as f32 - half,
        coord.y as f32 * CHUNK_SIZE as f32,
        coord.z as f32 * CHUNK_SIZE as f32 - half,
    ]
}

fn generate_chunk(coord: ChunkCoord) -> Chunk {
    let mut chunk = Chunk::new();
    let base_x = coord.x * CHUNK_SIZE as i32;
    let base_y = coord.y * CHUNK_SIZE as i32;
    let base_z = coord.z * CHUNK_SIZE as i32;

    for y in 0..CHUNK_SIZE {
        let world_y = base_y + y as i32;
        for z in 0..CHUNK_SIZE {
            let world_z = base_z + z as i32;
            for x in 0..CHUNK_SIZE {
                let world_x = base_x + x as i32;
                let height = terrain_height(world_x, world_z);

                if world_y <= height {
                    let color = block_color(world_y, height);
                    chunk.set(x, y, z, Block::Solid(color));
                }
            }
        }
    }

    chunk
}

fn terrain_height(x: i32, z: i32) -> i32 {
    let scale = 1.0 / 12.0;
    let fx = x as f32 * scale;
    let fz = z as f32 * scale;
    let hills = (fx * PI).sin() * 3.0 + (fz * PI * 0.5).cos() * 2.0;
    let base = 6.0;
    (base + hills).round() as i32
}

fn block_color(world_y: i32, surface_height: i32) -> [f32; 3] {
    if world_y == surface_height {
        [0.5, 0.7, 0.3]
    } else if world_y >= surface_height - 3 {
        [0.45, 0.35, 0.2]
    } else {
        [0.35, 0.35, 0.4]
    }
}

fn div_floor(a: i32, b: i32) -> i32 {
    let mut q = a / b;
    let r = a % b;
    if (r > 0 && b < 0) || (r < 0 && b > 0) {
        q -= 1;
    }
    q
}

fn mod_floor(a: i32, b: i32) -> i32 {
    let mut r = a % b;
    if (r > 0 && b < 0) || (r < 0 && b > 0) {
        r += b;
    }
    r
}
