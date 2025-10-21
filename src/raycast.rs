use glam::{IVec3, Vec3};

use crate::block::{BlockKind, FaceDirection};
use crate::world::World;

pub struct RaycastHit {
    pub block: IVec3,
    pub face: FaceDirection,
}

impl RaycastHit {
    pub fn placement_position(&self) -> IVec3 {
        self.block + self.face.normal()
    }
}

pub fn pick_block(
    world: &World,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<RaycastHit> {
    if max_distance <= 0.0 {
        return None;
    }

    let mut dir = direction;
    let len_sq = dir.length_squared();
    if len_sq < f32::EPSILON {
        return None;
    }
    if (len_sq - 1.0).abs() > 1e-6 {
        dir = dir.normalize();
    }

    let mut current = origin.floor().as_ivec3();
    let mut last_face: Option<FaceDirection> = None;
    let mut traveled = 0.0;
    let mut steps = 0;
    let max_steps = 512;

    let (step_x, mut t_max_x, t_delta_x) = axis_params(origin.x, dir.x, current.x);
    let (step_y, mut t_max_y, t_delta_y) = axis_params(origin.y, dir.y, current.y);
    let (step_z, mut t_max_z, t_delta_z) = axis_params(origin.z, dir.z, current.z);

    while traveled <= max_distance && steps < max_steps {
        if let Some(face) = last_face {
            if BlockKind::from_id(world.block_at(current.x, current.y, current.z)).is_solid() {
                return Some(RaycastHit {
                    block: current,
                    face,
                });
            }
        }

        // Choose next axis to step along.
        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                if step_x == 0 {
                    break;
                }
                current.x += step_x;
                traveled = t_max_x;
                t_max_x += t_delta_x;
                last_face = Some(if step_x > 0 {
                    FaceDirection::NegX
                } else {
                    FaceDirection::PosX
                });
            } else {
                if step_z == 0 {
                    break;
                }
                current.z += step_z;
                traveled = t_max_z;
                t_max_z += t_delta_z;
                last_face = Some(if step_z > 0 {
                    FaceDirection::NegZ
                } else {
                    FaceDirection::PosZ
                });
            }
        } else if t_max_y < t_max_z {
            if step_y == 0 {
                break;
            }
            current.y += step_y;
            traveled = t_max_y;
            t_max_y += t_delta_y;
            last_face = Some(if step_y > 0 {
                FaceDirection::NegY
            } else {
                FaceDirection::PosY
            });
        } else {
            if step_z == 0 {
                break;
            }
            current.z += step_z;
            traveled = t_max_z;
            t_max_z += t_delta_z;
            last_face = Some(if step_z > 0 {
                FaceDirection::NegZ
            } else {
                FaceDirection::PosZ
            });
        }

        steps += 1;
    }

    None
}

fn axis_params(
    origin_component: f32,
    direction_component: f32,
    block_component: i32,
) -> (i32, f32, f32) {
    if direction_component.abs() < f32::EPSILON {
        return (0, f32::INFINITY, f32::INFINITY);
    }

    let step = if direction_component > 0.0 { 1 } else { -1 };
    let boundary = if step > 0 {
        (block_component + 1) as f32
    } else {
        block_component as f32
    };

    let mut t_max = (boundary - origin_component) / direction_component;
    if t_max < 0.0 {
        t_max = 0.0;
    }
    let t_delta = 1.0 / direction_component.abs();
    (step, t_max, t_delta)
}
