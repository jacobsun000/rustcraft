use std::f32::consts::PI;

pub const CHUNK_SIZE: usize = 16;
const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

#[derive(Clone, Copy)]
pub enum Block {
    Air,
    Solid([f32; 3]),
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

pub struct MeshVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

pub fn generate_demo_chunk() -> Chunk {
    let mut chunk = Chunk::new();

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let fx = x as f32 / CHUNK_SIZE as f32;
                let fz = z as f32 / CHUNK_SIZE as f32;
                let height = ((fx * PI).sin() * (fz * PI).cos() * 4.0 + 6.0).round() as usize;

                if y <= height.min(CHUNK_SIZE - 1) {
                    let color = block_color(y);
                    chunk.set(x, y, z, Block::Solid(color));
                }
            }
        }
    }

    chunk
}

fn block_color(y: usize) -> [f32; 3] {
    if y > CHUNK_SIZE / 2 {
        [0.6, 0.8, 0.4]
    } else if y > CHUNK_SIZE / 4 {
        [0.5, 0.4, 0.25]
    } else {
        [0.4, 0.3, 0.2]
    }
}

pub fn build_mesh(chunk: &Chunk) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let offset = [-(CHUNK_SIZE as f32) / 2.0, 0.0, -(CHUNK_SIZE as f32) / 2.0];

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                if let Block::Solid(color) = chunk.get(x, y, z) {
                    add_block_faces(
                        [x as i32, y as i32, z as i32],
                        color,
                        chunk,
                        offset,
                        &mut vertices,
                        &mut indices,
                    );
                }
            }
        }
    }

    Mesh { vertices, indices }
}

fn add_block_faces(
    position: [i32; 3],
    color: [f32; 3],
    chunk: &Chunk,
    offset: [f32; 3],
    vertices: &mut Vec<MeshVertex>,
    indices: &mut Vec<u32>,
) {
    const FACES: [Face; 6] = [
        Face::new(
            [0, 0, -1],
            [
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
        ),
        Face::new(
            [0, 0, 1],
            [
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
            ],
        ),
        Face::new(
            [-1, 0, 0],
            [
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
            ],
        ),
        Face::new(
            [1, 0, 0],
            [
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
            ],
        ),
        Face::new(
            [0, -1, 0],
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
            ],
        ),
        Face::new(
            [0, 1, 0],
            [
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
            ],
        ),
    ];

    for face in FACES {
        let neighbor_pos = [
            position[0] + face.normal[0],
            position[1] + face.normal[1],
            position[2] + face.normal[2],
        ];

        if is_air(chunk, neighbor_pos) {
            let base_index = vertices.len() as u32;
            for vertex in face.vertices {
                let world_position = [
                    vertex[0] + position[0] as f32 + offset[0],
                    vertex[1] + position[1] as f32 + offset[1],
                    vertex[2] + position[2] as f32 + offset[2],
                ];
                vertices.push(MeshVertex {
                    position: world_position,
                    color,
                });
            }

            indices.extend_from_slice(&[
                base_index,
                base_index + 1,
                base_index + 2,
                base_index + 2,
                base_index + 1,
                base_index + 3,
            ]);
        }
    }
}

fn is_air(chunk: &Chunk, position: [i32; 3]) -> bool {
    if position
        .iter()
        .any(|&coord| coord < 0 || coord >= CHUNK_SIZE as i32)
    {
        return true;
    }

    match chunk.get(
        position[0] as usize,
        position[1] as usize,
        position[2] as usize,
    ) {
        Block::Air => true,
        Block::Solid(_) => false,
    }
}

struct Face {
    normal: [i32; 3],
    vertices: [[f32; 3]; 4],
}

impl Face {
    const fn new(normal: [i32; 3], vertices: [[f32; 3]; 4]) -> Self {
        Self { normal, vertices }
    }
}
