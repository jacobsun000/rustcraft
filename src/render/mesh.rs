use crate::texture::AtlasLayout;
use crate::world::{BlockId, BlockKind, CHUNK_SIZE, ChunkCoord, FaceDirection, World};

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

#[derive(Clone, Copy)]
struct BlockPosition {
    world: [i32; 3],
    origin: [f32; 3],
}

pub fn build_chunk_mesh(world: &World, coord: ChunkCoord, atlas: &AtlasLayout) -> Mesh {
    let chunk = world
        .chunk(coord)
        .expect("chunk must be generated before meshing");

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let chunk_origin = crate::world::chunk_origin(coord);
    let chunk_base = [
        coord.x * CHUNK_SIZE as i32,
        coord.y * CHUNK_SIZE as i32,
        coord.z * CHUNK_SIZE as i32,
    ];

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let block_id = chunk.get(x, y, z);
                if let Some(kind) = solid_kind(block_id) {
                    let world_position = [
                        chunk_base[0] + x as i32,
                        chunk_base[1] + y as i32,
                        chunk_base[2] + z as i32,
                    ];
                    let block_origin = [
                        chunk_origin[0] + x as f32,
                        chunk_origin[1] + y as f32,
                        chunk_origin[2] + z as f32,
                    ];
                    let block = BlockPosition {
                        world: world_position,
                        origin: block_origin,
                    };
                    add_block_faces(
                        world,
                        atlas,
                        kind,
                        block,
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
    block: BlockPosition,
    vertices: &mut Vec<MeshVertex>,
    indices: &mut Vec<u32>,
) {
    for face in FACES.iter() {
        let neighbor_world = [
            block.world[0] + face.normal[0],
            block.world[1] + face.normal[1],
            block.world[2] + face.normal[2],
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
                    block.origin[0] + corner[0],
                    block.origin[1] + corner[1],
                    block.origin[2] + corner[2],
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
