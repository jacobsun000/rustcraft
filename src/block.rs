use glam::IVec3;

use crate::texture::TileId;

pub type BlockId = u8;

pub const BLOCK_AIR: BlockId = 0;
pub const BLOCK_GRASS: BlockId = 1;
pub const BLOCK_DIRT: BlockId = 2;
pub const BLOCK_STONE: BlockId = 3;
pub const BLOCK_LAMP: BlockId = 4;
pub const BLOCK_GLASS: BlockId = 5;
pub const BLOCK_METAL: BlockId = 6;

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

    pub const fn normal(self) -> IVec3 {
        match self {
            FaceDirection::NegX => IVec3::new(-1, 0, 0),
            FaceDirection::PosX => IVec3::new(1, 0, 0),
            FaceDirection::NegY => IVec3::new(0, -1, 0),
            FaceDirection::PosY => IVec3::new(0, 1, 0),
            FaceDirection::NegZ => IVec3::new(0, 0, -1),
            FaceDirection::PosZ => IVec3::new(0, 0, 1),
        }
    }
}

#[derive(Clone, Copy)]
pub struct BlockDefinition {
    pub solid: bool,
    pub luminance: f32,
    pub specular: f32,
    pub diffuse: f32,
    pub roughness: f32,
    pub metallic: f32,
    pub transmission: f32,
    pub ior: f32,
    pub transmission_tint: f32,
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
    Metal,
    Glass,
}

impl BlockKind {
    pub const fn id(self) -> BlockId {
        match self {
            BlockKind::Air => BLOCK_AIR,
            BlockKind::Grass => BLOCK_GRASS,
            BlockKind::Dirt => BLOCK_DIRT,
            BlockKind::Stone => BLOCK_STONE,
            BlockKind::Lamp => BLOCK_LAMP,
            BlockKind::Metal => BLOCK_METAL,
            BlockKind::Glass => BLOCK_GLASS,
        }
    }

    pub fn from_id(id: BlockId) -> Self {
        match id {
            BLOCK_GRASS => BlockKind::Grass,
            BLOCK_DIRT => BlockKind::Dirt,
            BLOCK_STONE => BlockKind::Stone,
            BLOCK_LAMP => BlockKind::Lamp,
            BLOCK_METAL => BlockKind::Metal,
            BLOCK_GLASS => BlockKind::Glass,
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

    pub const fn display_name(self) -> &'static str {
        match self {
            BlockKind::Air => "Air",
            BlockKind::Grass => "Grass",
            BlockKind::Dirt => "Dirt",
            BlockKind::Stone => "Stone",
            BlockKind::Lamp => "Lamp",
            BlockKind::Metal => "Metal",
            BlockKind::Glass => "Glass",
        }
    }
}

pub fn block_definition(id: BlockId) -> &'static BlockDefinition {
    BlockKind::from_id(id).definition()
}

const TILE_GRASS_TOP: TileId = TileId { x: 0, y: 0 };
const TILE_GRASS_SIDE: TileId = TileId { x: 1, y: 0 };
const TILE_DIRT: TileId = TileId { x: 2, y: 0 };
const TILE_STONE: TileId = TileId { x: 3, y: 0 };
const TILE_LAMP: TileId = TileId { x: 4, y: 0 };
const TILE_AIR: TileId = TileId { x: 0, y: 0 };
const TILE_GLASS: TileId = TileId { x: 5, y: 0 };
const TILE_METAL: TileId = TileId { x: 6, y: 0 };

const BLOCK_DEFINITIONS: [BlockDefinition; 7] = [
    BlockDefinition {
        // Air
        solid: false,
        luminance: 0.0,
        specular: 0.0,
        diffuse: 0.0,
        roughness: 0.0,
        metallic: 0.0,
        transmission: 0.0,
        ior: 1.0,
        transmission_tint: 0.0,
        face_tiles: [TILE_AIR; 6],
    },
    BlockDefinition {
        // Grass
        solid: true,
        luminance: 0.0,
        specular: 0.04,
        diffuse: 0.85,
        roughness: 0.7,
        metallic: 0.0,
        transmission: 0.0,
        ior: 1.0,
        transmission_tint: 0.0,
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
        // Dirt
        solid: true,
        luminance: 0.0,
        specular: 0.025,
        diffuse: 0.75,
        roughness: 0.85,
        metallic: 0.0,
        transmission: 0.0,
        ior: 1.0,
        transmission_tint: 0.0,
        face_tiles: [TILE_DIRT; 6],
    },
    BlockDefinition {
        // Stone
        solid: true,
        luminance: 0.0,
        specular: 0.12,
        diffuse: 0.6,
        roughness: 0.45,
        metallic: 0.0,
        transmission: 0.0,
        ior: 1.0,
        transmission_tint: 0.0,
        face_tiles: [TILE_STONE; 6],
    },
    BlockDefinition {
        // Lamp
        solid: true,
        luminance: 8.0,
        specular: 0.08,
        diffuse: 0.9,
        roughness: 0.6,
        metallic: 0.0,
        transmission: 0.0,
        ior: 1.2,
        transmission_tint: 0.0,
        face_tiles: [TILE_LAMP; 6],
    },
    BlockDefinition {
        // Metal
        solid: true,
        luminance: 0.0,
        specular: 0.9,
        diffuse: 0.15,
        roughness: 0.2,
        metallic: 1.0,
        transmission: 0.0,
        ior: 1.0,
        transmission_tint: 0.0,
        face_tiles: [TILE_METAL; 6],
    },
    BlockDefinition {
        // Glass
        solid: true,
        luminance: 0.0,
        specular: 0.06,
        diffuse: 0.05,
        roughness: 0.05,
        metallic: 0.0,
        transmission: 0.95,
        ior: 1.45,
        transmission_tint: 0.85,
        face_tiles: [TILE_GLASS; 6],
    },
];
