use std::{collections::HashMap, f32::consts::PI};

use glam::IVec3;

use crate::block::{BLOCK_AIR, BlockId, BlockKind};

pub const CHUNK_SIZE: usize = 16;
const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

pub struct Chunk {
    blocks: Vec<BlockId>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            blocks: vec![BLOCK_AIR; CHUNK_VOLUME],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, block: BlockId) {
        let index = Self::index(x, y, z);
        self.blocks[index] = block;
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockId {
        let index = Self::index(x, y, z);
        self.blocks[index]
    }

    pub fn blocks(&self) -> &[BlockId] {
        &self.blocks
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

    pub fn block_at(&self, world_x: i32, world_y: i32, world_z: i32) -> BlockId {
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
            .unwrap_or(BLOCK_AIR)
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn iter_chunks(&self) -> impl Iterator<Item = (&ChunkCoord, &Chunk)> {
        self.chunks.iter()
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

pub fn chunk_min_corner(coord: ChunkCoord) -> IVec3 {
    IVec3::new(
        coord.x * CHUNK_SIZE as i32,
        coord.y * CHUNK_SIZE as i32,
        coord.z * CHUNK_SIZE as i32,
    )
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
                    let kind = if world_y == height {
                        BlockKind::Grass
                    } else if world_y >= height - 3 {
                        BlockKind::Dirt
                    } else {
                        BlockKind::Stone
                    };
                    chunk.set(x, y, z, kind.id());
                }
            }
        }
    }

    if coord == (ChunkCoord { x: 0, y: 0, z: 0 }) {
        let lamp_x = CHUNK_SIZE / 2;
        let lamp_z = CHUNK_SIZE / 2;
        let world_x = base_x + lamp_x as i32;
        let world_z = base_z + lamp_z as i32;
        let lamp_world_y = terrain_height(world_x, world_z) + 1;
        if lamp_world_y >= base_y && lamp_world_y < base_y + CHUNK_SIZE as i32 {
            let lamp_y = (lamp_world_y - base_y) as usize;
            chunk.set(lamp_x, lamp_y, lamp_z, BlockKind::Lamp.id());
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
