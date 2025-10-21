use glam::Vec3;

use crate::block::BlockKind;
use crate::input::MovementInput;
use crate::world::World;

const PLAYER_WIDTH: f32 = 0.6;
const PLAYER_HALF_WIDTH: f32 = PLAYER_WIDTH * 0.5;
const PLAYER_HEIGHT: f32 = 1.8;
pub const PLAYER_EYE_HEIGHT: f32 = 1.62;

const FLY_SPEED_MULTIPLIER: f32 = 1.0;
const WALK_SPEED: f32 = 4.5;
const JUMP_SPEED: f32 = 6.5;
const GRAVITY: f32 = -20.0;
const MAX_FALL_SPEED: f32 = -54.0;
const COLLISION_STEP: f32 = 0.25;
const COLLISION_EPS: f32 = 1e-4;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MovementMode {
    Fly,
    Walk,
}

impl MovementMode {
    pub fn toggle(self) -> Self {
        match self {
            MovementMode::Fly => MovementMode::Walk,
            MovementMode::Walk => MovementMode::Fly,
        }
    }
}

pub struct PlayerPhysics {
    position: Vec3,
    velocity: Vec3,
    mode: MovementMode,
    on_ground: bool,
}

impl PlayerPhysics {
    pub fn new(feet_position: Vec3, mode: MovementMode) -> Self {
        Self {
            position: feet_position,
            velocity: Vec3::ZERO,
            mode,
            on_ground: false,
        }
    }

    pub fn from_camera(camera_position: Vec3) -> Self {
        let feet = camera_position - Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);
        Self::new(feet, MovementMode::Walk)
    }

    pub fn camera_position(&self) -> Vec3 {
        self.position + Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0)
    }

    pub fn mode(&self) -> MovementMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: MovementMode) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        if matches!(self.mode, MovementMode::Fly) {
            self.on_ground = false;
        } else {
            self.velocity.y = 0.0;
        }
    }

    pub fn toggle_mode(&mut self) {
        let new_mode = self.mode.toggle();
        self.set_mode(new_mode);
    }

    pub fn update(&mut self, world: &World, dt: f32, movement: &MovementInput) {
        match self.mode {
            MovementMode::Fly => self.update_fly(world, dt, movement),
            MovementMode::Walk => self.update_walk(world, dt, movement),
        }
    }

    fn update_fly(&mut self, world: &World, dt: f32, movement: &MovementInput) {
        let mut desired = movement.wish_dir;
        if movement.ascend {
            desired += Vec3::Y;
        }
        if movement.descend {
            desired -= Vec3::Y;
        }

        if desired.length_squared() > 0.0 {
            self.velocity = desired.normalize() * (movement.speed * FLY_SPEED_MULTIPLIER);
        } else {
            self.velocity = Vec3::ZERO;
        }

        self.apply_movement(world, dt);
    }

    fn update_walk(&mut self, world: &World, dt: f32, movement: &MovementInput) {
        let mut desired = movement.wish_dir;
        desired.y = 0.0;
        if desired.length_squared() > 0.0 {
            desired = desired.normalize() * WALK_SPEED;
        }

        self.velocity.x = desired.x;
        self.velocity.z = desired.z;

        if movement.jump && self.on_ground {
            self.velocity.y = JUMP_SPEED;
            self.on_ground = false;
        } else {
            self.velocity.y += GRAVITY * dt;
            if self.velocity.y < MAX_FALL_SPEED {
                self.velocity.y = MAX_FALL_SPEED;
            }
        }

        self.apply_movement(world, dt);
    }

    fn apply_movement(&mut self, world: &World, dt: f32) {
        let dx = self.velocity.x * dt;
        let dy = self.velocity.y * dt;
        let dz = self.velocity.z * dt;

        self.move_along_axis(world, Axis::X, dx);
        let vertical_hit = self.move_along_axis(world, Axis::Y, dy);
        self.move_along_axis(world, Axis::Z, dz);

        if let Some(hit) = vertical_hit {
            if hit == VerticalHit::Floor {
                self.on_ground = true;
                self.velocity.y = 0.0;
            } else {
                self.velocity.y = 0.0;
            }
        } else if dy.abs() > 0.0 {
            // If we moved vertically without a hit, we are airborne.
            if dy < 0.0 {
                self.on_ground = false;
            }
        }
    }

    fn move_along_axis(&mut self, world: &World, axis: Axis, delta: f32) -> Option<VerticalHit> {
        if delta.abs() < f32::EPSILON {
            return None;
        }

        let mut remaining = delta;
        let mut last_vertical_hit = None;

        while remaining.abs() > f32::EPSILON {
            let step = remaining.clamp(-COLLISION_STEP, COLLISION_STEP);
            let candidate = self.position_with_axis_offset(axis, step);

            if self.collides(world, candidate) {
                // Increase precision near the collision.
                let mut reduced = step;
                while reduced.abs() > COLLISION_EPS {
                    reduced *= 0.5;
                    let refined = self.position_with_axis_offset(axis, reduced);
                    if !self.collides(world, refined) {
                        self.position = refined;
                        break;
                    }
                }

                match axis {
                    Axis::X => self.velocity.x = 0.0,
                    Axis::Y => {
                        if delta < 0.0 {
                            last_vertical_hit = Some(VerticalHit::Floor);
                        } else {
                            last_vertical_hit = Some(VerticalHit::Ceiling);
                        }
                    }
                    Axis::Z => self.velocity.z = 0.0,
                }
                break;
            } else {
                self.position = candidate;
                remaining -= step;
            }
        }

        last_vertical_hit
    }

    fn position_with_axis_offset(&self, axis: Axis, delta: f32) -> Vec3 {
        match axis {
            Axis::X => Vec3::new(self.position.x + delta, self.position.y, self.position.z),
            Axis::Y => Vec3::new(self.position.x, self.position.y + delta, self.position.z),
            Axis::Z => Vec3::new(self.position.x, self.position.y, self.position.z + delta),
        }
    }

    fn collides(&self, world: &World, feet_position: Vec3) -> bool {
        let min_x = feet_position.x - PLAYER_HALF_WIDTH;
        let max_x = feet_position.x + PLAYER_HALF_WIDTH;
        let min_y = feet_position.y;
        let max_y = feet_position.y + PLAYER_HEIGHT;
        let min_z = feet_position.z - PLAYER_HALF_WIDTH;
        let max_z = feet_position.z + PLAYER_HALF_WIDTH;

        let min_block_x = min_x.floor() as i32;
        let max_block_x = (max_x - COLLISION_EPS).floor() as i32;
        let min_block_y = min_y.floor() as i32;
        let max_block_y = (max_y - COLLISION_EPS).floor() as i32;
        let min_block_z = min_z.floor() as i32;
        let max_block_z = (max_z - COLLISION_EPS).floor() as i32;

        for y in min_block_y..=max_block_y {
            for z in min_block_z..=max_block_z {
                for x in min_block_x..=max_block_x {
                    if BlockKind::from_id(world.block_at(x, y, z)).is_solid() {
                        return true;
                    }
                }
            }
        }

        false
    }
}

#[derive(Copy, Clone)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum VerticalHit {
    Floor,
    Ceiling,
}
