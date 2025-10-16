use std::{collections::HashMap, f32::consts::PI};

use glam::IVec3;

use crate::texture::TileId;

pub const CHUNK_SIZE: usize = 16;
const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

pub type BlockId = u8;

pub const BLOCK_AIR: BlockId = 0;
pub const BLOCK_GRASS: BlockId = 1;
pub const BLOCK_DIRT: BlockId = 2;
pub const BLOCK_STONE: BlockId = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Air,
    Grass,
    Dirt,
    Stone,
}

impl BlockKind {
    pub fn id(self) -> BlockId {
        match self {
            BlockKind::Air => BLOCK_AIR,
            BlockKind::Grass => BLOCK_GRASS,
            BlockKind::Dirt => BLOCK_DIRT,
            BlockKind::Stone => BLOCK_STONE,
        }
    }

    pub fn from_id(id: BlockId) -> Self {
        match id {
            BLOCK_GRASS => BlockKind::Grass,
            BLOCK_DIRT => BlockKind::Dirt,
            BLOCK_STONE => BlockKind::Stone,
            _ => BlockKind::Air,
        }
    }

    pub fn is_solid(self) -> bool {
        !matches!(self, BlockKind::Air)
    }

    pub fn tile_for_face(self, face: FaceDirection) -> TileId {
        match self {
            BlockKind::Grass => match face {
                FaceDirection::PosY => TILE_GRASS_TOP,
                FaceDirection::NegY => TILE_DIRT,
                _ => TILE_GRASS_SIDE,
            },
            BlockKind::Dirt => TILE_DIRT,
            BlockKind::Stone => TILE_STONE,
            BlockKind::Air => TILE_AIR,
        }
    }

    #[allow(dead_code)]
    pub fn diffuse_color(self, face: FaceDirection) -> [f32; 3] {
        match self {
            BlockKind::Grass => match face {
                FaceDirection::PosY => [0.58, 0.78, 0.32],
                FaceDirection::NegY => [0.38, 0.25, 0.14],
                _ => [0.32, 0.56, 0.24],
            },
            BlockKind::Dirt => [0.42, 0.27, 0.16],
            BlockKind::Stone => [0.55, 0.55, 0.58],
            BlockKind::Air => [0.0, 0.0, 0.0],
        }
    }
}

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

#[derive(Clone, Copy)]
pub enum FaceDirection {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
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

const TILE_GRASS_TOP: TileId = TileId { x: 0, y: 0 };
const TILE_GRASS_SIDE: TileId = TileId { x: 1, y: 0 };
const TILE_DIRT: TileId = TileId { x: 2, y: 0 };
const TILE_STONE: TileId = TileId { x: 3, y: 0 };
const TILE_AIR: TileId = TileId { x: 0, y: 0 };

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
