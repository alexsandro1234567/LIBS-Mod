//! ECS Components for Minecraft entities
//!
//! DOD (Data-Oriented Design) versions of Minecraft entity data

use super::{Component, ComponentId};

/// Position component
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Component for Position {
    fn type_id() -> ComponentId { 1 }
}

/// Velocity component
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Component for Velocity {
    fn type_id() -> ComponentId { 2 }
}

/// Health component
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Health {
    pub current: f32,
    pub max: f32,
    pub regeneration: f32,
}

impl Default for Health {
    fn default() -> Self {
        Self { current: 20.0, max: 20.0, regeneration: 0.0 }
    }
}

impl Component for Health {
    fn type_id() -> ComponentId { 3 }
}

/// Collision component
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Collision {
    pub width: f32,
    pub height: f32,
    pub on_ground: bool,
    pub no_clip: bool,
}

impl Component for Collision {
    fn type_id() -> ComponentId { 4 }
}

/// AI State component
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AiState {
    pub state: AiBehavior,
    pub target_entity: u32,
    pub target_pos: [f32; 3],
    pub state_timer: f32,
}

#[repr(u8)]
#[derive(Clone, Copy, Default)]
pub enum AiBehavior {
    #[default]
    Idle = 0,
    Wander = 1,
    Chase = 2,
    Attack = 3,
    Flee = 4,
    Follow = 5,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            state: AiBehavior::Idle,
            target_entity: 0,
            target_pos: [0.0; 3],
            state_timer: 0.0,
        }
    }
}

impl Component for AiState {
    fn type_id() -> ComponentId { 5 }
}

/// Render component
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Render {
    pub model_id: u32,
    pub texture_id: u32,
    pub animation_frame: u16,
    pub animation_speed: f32,
    pub visible: bool,
}

impl Default for Render {
    fn default() -> Self {
        Self {
            model_id: 0,
            texture_id: 0,
            animation_frame: 0,
            animation_speed: 1.0,
            visible: true,
        }
    }
}

impl Component for Render {
    fn type_id() -> ComponentId { 6 }
}

/// Physics component
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Physics {
    pub mass: f32,
    pub drag: f32,
    pub gravity_scale: f32,
    pub restitution: f32,
}

impl Default for Physics {
    fn default() -> Self {
        Self {
            mass: 1.0,
            drag: 0.02,
            gravity_scale: 1.0,
            restitution: 0.0,
        }
    }
}

impl Component for Physics {
    fn type_id() -> ComponentId { 7 }
}

/// Inventory component
#[repr(C)]
#[derive(Clone)]
pub struct Inventory {
    pub slots: Vec<ItemStack>,
    pub selected_slot: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ItemStack {
    pub item_id: u32,
    pub count: u8,
    pub damage: u16,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: vec![ItemStack::default(); 36],
            selected_slot: 0,
        }
    }
}

impl Component for Inventory {
    fn type_id() -> ComponentId { 8 }
}

/// Entity type marker
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EntityType {
    pub type_id: u16,
    pub is_player: bool,
    pub is_hostile: bool,
    pub is_passive: bool,
}

impl Default for EntityType {
    fn default() -> Self {
        Self {
            type_id: 0,
            is_player: false,
            is_hostile: false,
            is_passive: true,
        }
    }
}

impl Component for EntityType {
    fn type_id() -> ComponentId { 9 }
}
