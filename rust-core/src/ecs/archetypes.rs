//! # ECS Archetypes
//! 
//! Predefined entity archetypes for common entity types.

use super::components::*;

/// Player entity archetype
pub struct PlayerArchetype {
    pub transform: Transform,
    pub velocity: Velocity,
    pub physics: PhysicsBody,
    pub health: Health,
    pub bounds: Bounds,
    pub visibility: Visibility,
    pub mc_entity: MinecraftEntity,
    pub player: Player,
}

impl PlayerArchetype {
    pub fn new(mc_id: i32, x: f64, y: f64, z: f64) -> Self {
        Self {
            transform: Transform::new(x, y, z),
            velocity: Velocity::default(),
            physics: PhysicsBody {
                mass: 70.0,
                drag: 0.02,
                gravity_scale: 1.0,
                grounded: false,
            },
            health: Health::new(20.0),
            bounds: Bounds::new(0.6, 1.8, 0.6),
            visibility: Visibility {
                visible: true,
                render_distance: 128.0,
            },
            mc_entity: MinecraftEntity {
                mc_id,
                entity_type: 0,
            },
            player: Player,
        }
    }
    
    pub fn into_bundle(self) -> impl hecs::DynamicBundle {
        (
            self.transform,
            self.velocity,
            self.physics,
            self.health,
            self.bounds,
            self.visibility,
            self.mc_entity,
            self.player,
        )
    }
}

/// Monster entity archetype
pub struct MonsterArchetype {
    pub transform: Transform,
    pub velocity: Velocity,
    pub physics: PhysicsBody,
    pub health: Health,
    pub bounds: Bounds,
    pub visibility: Visibility,
    pub mc_entity: MinecraftEntity,
    pub ai: AIState,
    pub monster: Monster,
}

impl MonsterArchetype {
    pub fn new(mc_id: i32, entity_type: i32, x: f64, y: f64, z: f64, max_health: f32) -> Self {
        Self {
            transform: Transform::new(x, y, z),
            velocity: Velocity::default(),
            physics: PhysicsBody::default(),
            health: Health::new(max_health),
            bounds: Bounds::new(0.6, 1.8, 0.6),
            visibility: Visibility::default(),
            mc_entity: MinecraftEntity { mc_id, entity_type },
            ai: AIState::default(),
            monster: Monster,
        }
    }
    
    pub fn into_bundle(self) -> impl hecs::DynamicBundle {
        (
            self.transform,
            self.velocity,
            self.physics,
            self.health,
            self.bounds,
            self.visibility,
            self.mc_entity,
            self.ai,
            self.monster,
        )
    }
}

/// Item entity archetype
pub struct ItemArchetype {
    pub transform: Transform,
    pub velocity: Velocity,
    pub physics: PhysicsBody,
    pub bounds: Bounds,
    pub visibility: Visibility,
    pub mc_entity: MinecraftEntity,
    pub item: Item,
}

impl ItemArchetype {
    pub fn new(mc_id: i32, x: f64, y: f64, z: f64) -> Self {
        Self {
            transform: Transform::new(x, y, z),
            velocity: Velocity::default(),
            physics: PhysicsBody {
                mass: 0.1,
                drag: 0.04,
                gravity_scale: 1.0,
                grounded: false,
            },
            bounds: Bounds::new(0.25, 0.25, 0.25),
            visibility: Visibility {
                visible: true,
                render_distance: 32.0,
            },
            mc_entity: MinecraftEntity {
                mc_id,
                entity_type: 2,
            },
            item: Item,
        }
    }
    
    pub fn into_bundle(self) -> impl hecs::DynamicBundle {
        (
            self.transform,
            self.velocity,
            self.physics,
            self.bounds,
            self.visibility,
            self.mc_entity,
            self.item,
        )
    }
}
