use std::{
    collections::{HashMap, hash_map::Entry},
    f32::consts::PI,
};

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
    visible_mask: Vec<bool>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            blocks: vec![BLOCK_AIR; CHUNK_VOLUME],
            visible_mask: vec![false; CHUNK_VOLUME],
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

    pub fn visible_mask(&self) -> &[bool] {
        &self.visible_mask
    }

    pub fn set_visible_mask(&mut self, mask: Vec<bool>) {
        debug_assert_eq!(mask.len(), CHUNK_VOLUME);
        self.visible_mask = mask;
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
        let mut inserted = false;
        match self.chunks.entry(coord) {
            Entry::Occupied(_) => {}
            Entry::Vacant(vacant) => {
                let chunk = generate_chunk(coord);
                vacant.insert(chunk);
                inserted = true;
            }
        }

        if inserted {
            self.recompute_visibility_around(coord);
        }
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

    fn recompute_visibility_around(&mut self, center: ChunkCoord) {
        let offsets = [
            IVec3::new(0, 0, 0),
            IVec3::new(1, 0, 0),
            IVec3::new(-1, 0, 0),
            IVec3::new(0, 1, 0),
            IVec3::new(0, -1, 0),
            IVec3::new(0, 0, 1),
            IVec3::new(0, 0, -1),
        ];

        for offset in offsets {
            let neighbor_coord = ChunkCoord {
                x: center.x + offset.x,
                y: center.y + offset.y,
                z: center.z + offset.z,
            };

            if self.chunks.contains_key(&neighbor_coord) {
                if let Some(mask) = self.compute_visibility_mask(neighbor_coord) {
                    if let Some(chunk) = self.chunks.get_mut(&neighbor_coord) {
                        chunk.set_visible_mask(mask);
                    }
                }
            }
        }
    }

    fn compute_visibility_mask(&self, coord: ChunkCoord) -> Option<Vec<bool>> {
        let chunk = self.chunk(coord)?;
        let base = chunk_min_corner(coord);
        let mut mask = vec![false; CHUNK_VOLUME];

        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let index = Chunk::index(x, y, z);
                    let block = chunk.blocks()[index];
                    let kind = BlockKind::from_id(block);
                    if !kind.is_solid() {
                        continue;
                    }

                    let world_pos = base + IVec3::new(x as i32, y as i32, z as i32);
                    if self.block_has_exposed_face(world_pos) {
                        mask[index] = true;
                    }
                }
            }
        }

        Some(mask)
    }

    fn block_has_exposed_face(&self, position: IVec3) -> bool {
        const NEIGHBORS: [IVec3; 6] = [
            IVec3::new(1, 0, 0),
            IVec3::new(-1, 0, 0),
            IVec3::new(0, 1, 0),
            IVec3::new(0, -1, 0),
            IVec3::new(0, 0, 1),
            IVec3::new(0, 0, -1),
        ];

        for offset in NEIGHBORS {
            let neighbor_pos = position + offset;
            let block = self.block_at(neighbor_pos.x, neighbor_pos.y, neighbor_pos.z);
            if !BlockKind::from_id(block).is_solid() {
                return true;
            }
        }

        false
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

pub fn chunk_coord_from_block(position: IVec3) -> ChunkCoord {
    ChunkCoord {
        x: div_floor(position.x, CHUNK_SIZE as i32),
        y: div_floor(position.y, CHUNK_SIZE as i32),
        z: div_floor(position.z, CHUNK_SIZE as i32),
    }
}

impl World {
    pub fn ensure_chunks_in_radius(
        &mut self,
        center: ChunkCoord,
        radius: i32,
        vertical_radius: i32,
    ) {
        for dy in -vertical_radius..=vertical_radius {
            for dz in -radius..=radius {
                for dx in -radius..=radius {
                    let coord = ChunkCoord {
                        x: center.x + dx,
                        y: center.y + dy,
                        z: center.z + dz,
                    };
                    self.ensure_chunk(coord);
                }
            }
        }
    }
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
