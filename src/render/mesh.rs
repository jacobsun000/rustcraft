use crate::texture::AtlasLayout;
use crate::world::{self, BlockId, BlockKind, CHUNK_SIZE, ChunkCoord, FaceDirection, World};

#[derive(Clone, Copy)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
}

pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

pub fn build_chunk_mesh(world: &World, coord: ChunkCoord, atlas: &AtlasLayout) -> Mesh {
    let chunk = world
        .chunk(coord)
        .expect("chunk must be generated before meshing");

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let origin = world::chunk_origin(coord);

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let block_id = chunk.get(x, y, z);
                if let Some(kind) = solid_kind(block_id) {
                    add_block_faces(
                        world,
                        atlas,
                        kind,
                        [
                            coord.x * CHUNK_SIZE as i32 + x as i32,
                            coord.y * CHUNK_SIZE as i32 + y as i32,
                            coord.z * CHUNK_SIZE as i32 + z as i32,
                        ],
                        origin,
                        [x as f32, y as f32, z as f32],
                        &mut vertices,
                        &mut indices,
                    );
                }
            }
        }
    }

    Mesh { vertices, indices }
}

fn solid_kind(id: BlockId) -> Option<BlockKind> {
    let kind = BlockKind::from_id(id);
    if kind.is_solid() { Some(kind) } else { None }
}

fn add_block_faces(
    world: &World,
    atlas: &AtlasLayout,
    kind: BlockKind,
    world_position: [i32; 3],
    chunk_origin: [f32; 3],
    block_offset: [f32; 3],
    vertices: &mut Vec<MeshVertex>,
    indices: &mut Vec<u32>,
) {
    for face in FACES.iter() {
        let neighbor_world = [
            world_position[0] + face.normal[0],
            world_position[1] + face.normal[1],
            world_position[2] + face.normal[2],
        ];

        let neighbor_block =
            world.block_at(neighbor_world[0], neighbor_world[1], neighbor_world[2]);

        if !BlockKind::from_id(neighbor_block).is_solid() {
            let tile = kind.tile_for_face(face.direction);
            let shade = face.light;
            let color = [shade, shade, shade];

            let base_index = vertices.len() as u32;
            for (corner, uv) in face.vertices.iter().zip(face.uvs.iter()) {
                let position = [
                    chunk_origin[0] + block_offset[0] + corner[0],
                    chunk_origin[1] + block_offset[1] + corner[1],
                    chunk_origin[2] + block_offset[2] + corner[2],
                ];
                let tex_uv = atlas.map_uv(tile, *uv);
                vertices.push(MeshVertex {
                    position,
                    color,
                    uv: tex_uv,
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

struct Face {
    normal: [i32; 3],
    vertices: [[f32; 3]; 4],
    uvs: [[f32; 2]; 4],
    direction: FaceDirection,
    light: f32,
}

impl Face {
    const fn new(
        normal: [i32; 3],
        vertices: [[f32; 3]; 4],
        uvs: [[f32; 2]; 4],
        direction: FaceDirection,
        light: f32,
    ) -> Self {
        Self {
            normal,
            vertices,
            uvs,
            direction,
            light,
        }
    }
}

const FACES: [Face; 6] = [
    Face::new(
        [0, 0, -1],
        [
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]],
        FaceDirection::NegZ,
        0.85,
    ),
    Face::new(
        [0, 0, 1],
        [
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
        ],
        [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
        FaceDirection::PosZ,
        0.85,
    ),
    Face::new(
        [-1, 0, 0],
        [
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
        ],
        [[1.0, 0.0], [0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        FaceDirection::NegX,
        0.75,
    ),
    Face::new(
        [1, 0, 0],
        [
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
        ],
        [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]],
        FaceDirection::PosX,
        0.75,
    ),
    Face::new(
        [0, -1, 0],
        [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
        ],
        [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
        FaceDirection::NegY,
        0.6,
    ),
    Face::new(
        [0, 1, 0],
        [
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
        ],
        [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]],
        FaceDirection::PosY,
        1.0,
    ),
];
