use crate::world::{Block, CHUNK_SIZE, ChunkCoord, World, chunk_origin};

#[derive(Clone, Copy)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

pub fn build_chunk_mesh(world: &World, coord: ChunkCoord) -> Mesh {
    let chunk = world
        .chunk(coord)
        .expect("chunk must be generated before meshing");

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let origin = chunk_origin(coord);

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                if let Block::Solid(color) = chunk.get(x, y, z) {
                    add_block_faces(
                        world,
                        [
                            coord.x * CHUNK_SIZE as i32 + x as i32,
                            coord.y * CHUNK_SIZE as i32 + y as i32,
                            coord.z * CHUNK_SIZE as i32 + z as i32,
                        ],
                        origin,
                        [x as f32, y as f32, z as f32],
                        color,
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
    world: &World,
    world_position: [i32; 3],
    chunk_origin: [f32; 3],
    block_offset: [f32; 3],
    color: [f32; 3],
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
        let neighbor_world = [
            world_position[0] + face.normal[0],
            world_position[1] + face.normal[1],
            world_position[2] + face.normal[2],
        ];

        if matches!(
            world.block_at(neighbor_world[0], neighbor_world[1], neighbor_world[2]),
            Block::Air
        ) {
            let base_index = vertices.len() as u32;
            for vertex in face.vertices {
                let position = [
                    chunk_origin[0] + block_offset[0] + vertex[0],
                    chunk_origin[1] + block_offset[1] + vertex[1],
                    chunk_origin[2] + block_offset[2] + vertex[2],
                ];
                vertices.push(MeshVertex { position, color });
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

struct Face {
    normal: [i32; 3],
    vertices: [[f32; 3]; 4],
}

impl Face {
    const fn new(normal: [i32; 3], vertices: [[f32; 3]; 4]) -> Self {
        Self { normal, vertices }
    }
}
