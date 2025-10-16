use crate::texture::TileId;

pub type BlockId = u8;

pub const BLOCK_AIR: BlockId = 0;
pub const BLOCK_GRASS: BlockId = 1;
pub const BLOCK_DIRT: BlockId = 2;
pub const BLOCK_STONE: BlockId = 3;
pub const BLOCK_LAMP: BlockId = 4;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceDirection {
    NegX = 0,
    PosX = 1,
    NegY = 2,
    PosY = 3,
    NegZ = 4,
    PosZ = 5,
}

impl FaceDirection {
    pub const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy)]
pub struct BlockDefinition {
    pub solid: bool,
    pub luminance: f32,
    pub reflectivity: f32,
    pub face_tiles: [TileId; 6],
}

impl BlockDefinition {
    pub const fn tile_for_face(&self, face: FaceDirection) -> TileId {
        self.face_tiles[face.index()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Air,
    Grass,
    Dirt,
    Stone,
    Lamp,
}

impl BlockKind {
    pub const fn id(self) -> BlockId {
        match self {
            BlockKind::Air => BLOCK_AIR,
            BlockKind::Grass => BLOCK_GRASS,
            BlockKind::Dirt => BLOCK_DIRT,
            BlockKind::Stone => BLOCK_STONE,
            BlockKind::Lamp => BLOCK_LAMP,
        }
    }

    pub fn from_id(id: BlockId) -> Self {
        match id {
            BLOCK_GRASS => BlockKind::Grass,
            BLOCK_DIRT => BlockKind::Dirt,
            BLOCK_STONE => BlockKind::Stone,
            BLOCK_LAMP => BlockKind::Lamp,
            _ => BlockKind::Air,
        }
    }

    pub fn definition(self) -> &'static BlockDefinition {
        &BLOCK_DEFINITIONS[self.id() as usize]
    }

    pub fn is_solid(self) -> bool {
        self.definition().solid
    }

    pub fn tile_for_face(self, face: FaceDirection) -> TileId {
        self.definition().tile_for_face(face)
    }
}

pub fn block_definition(id: BlockId) -> &'static BlockDefinition {
    BlockKind::from_id(id).definition()
}

const TILE_GRASS_TOP: TileId = TileId { x: 0, y: 0 };
const TILE_GRASS_SIDE: TileId = TileId { x: 1, y: 0 };
const TILE_DIRT: TileId = TileId { x: 2, y: 0 };
const TILE_STONE: TileId = TileId { x: 3, y: 0 };
const TILE_LAMP: TileId = TileId { x: 3, y: 0 };
const TILE_AIR: TileId = TileId { x: 0, y: 0 };

const BLOCK_DEFINITIONS: [BlockDefinition; 5] = [
    BlockDefinition {
        solid: false,
        luminance: 0.0,
        reflectivity: 0.0,
        face_tiles: [TILE_AIR; 6],
    },
    BlockDefinition {
        solid: true,
        luminance: 0.0,
        reflectivity: 0.05,
        face_tiles: [
            TILE_GRASS_SIDE,
            TILE_GRASS_SIDE,
            TILE_DIRT,
            TILE_GRASS_TOP,
            TILE_GRASS_SIDE,
            TILE_GRASS_SIDE,
        ],
    },
    BlockDefinition {
        solid: true,
        luminance: 0.0,
        reflectivity: 0.02,
        face_tiles: [TILE_DIRT; 6],
    },
    BlockDefinition {
        solid: true,
        luminance: 0.0,
        reflectivity: 0.2,
        face_tiles: [TILE_STONE; 6],
    },
    BlockDefinition {
        solid: true,
        luminance: 8.0,
        reflectivity: 0.0,
        face_tiles: [TILE_LAMP; 6],
    },
];
