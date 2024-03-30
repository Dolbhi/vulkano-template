use crate::game_objects::{transform::TransformID, utility::IDCollection};

pub struct CubiodCollider {
    transform: TransformID,
}

pub struct ColliderSystem {
    pub cuboid_colliders: IDCollection<CubiodCollider>,
}
