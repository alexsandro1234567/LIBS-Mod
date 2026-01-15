//! # ECS Queries
//! 
//! Predefined queries for common entity lookups.

use super::components::*;

/// Query for all renderable entities
pub type RenderableQuery<'a> = (&'a Transform, &'a RenderData, &'a Visibility);

/// Query for all physics entities
pub type PhysicsQuery<'a> = (&'a mut Transform, &'a mut Velocity, &'a PhysicsBody);

/// Query for all animated entities
pub type AnimatedQuery<'a> = (&'a Transform, &'a mut AnimationState, &'a RenderData);

/// Query for all AI-controlled entities
pub type AIQuery<'a> = (&'a Transform, &'a mut Velocity, &'a mut AIState);

/// Query for all entities with health
pub type HealthQuery<'a> = (&'a Transform, &'a mut Health);

/// Query for player entities
pub type PlayerQuery<'a> = (&'a Transform, &'a Velocity, &'a Player);

/// Query for monster entities
pub type MonsterQuery<'a> = (&'a Transform, &'a Velocity, &'a Health, &'a Monster);

/// Query for item entities
pub type ItemQuery<'a> = (&'a Transform, &'a Item);

/// Query for projectile entities
pub type ProjectileQuery<'a> = (&'a Transform, &'a Velocity, &'a Projectile);

/// Query for particle emitters
pub type ParticleEmitterQuery<'a> = (&'a Transform, &'a mut ParticleEmitter);

/// Query for sound emitters
pub type SoundEmitterQuery<'a> = (&'a Transform, &'a SoundEmitter);

/// Spatial query result
#[derive(Debug, Clone)]
pub struct SpatialQueryResult {
    pub entity_handle: u64,
    pub position: Position,
    pub distance: f64,
}

/// Spatial query helper for finding nearby entities
pub struct SpatialQuery {
    /// Grid cell size
    cell_size: f64,
    /// Spatial hash grid
    grid: std::collections::HashMap<(i32, i32, i32), Vec<u64>>,
}

impl SpatialQuery {
    /// Create a new spatial query helper
    pub fn new(cell_size: f64) -> Self {
        Self {
            cell_size,
            grid: std::collections::HashMap::new(),
        }
    }
    
    /// Clear the spatial grid
    pub fn clear(&mut self) {
        self.grid.clear();
    }
    
    /// Insert an entity into the spatial grid
    pub fn insert(&mut self, handle: u64, position: &Position) {
        let cell = self.position_to_cell(position);
        self.grid.entry(cell).or_insert_with(Vec::new).push(handle);
    }
    
    /// Query entities within radius of a position
    pub fn query_radius(&self, center: &Position, radius: f64) -> Vec<u64> {
        let mut results = Vec::new();
        
        // Calculate cell range to check
        let cells_to_check = (radius / self.cell_size).ceil() as i32 + 1;
        let center_cell = self.position_to_cell(center);
        
        for dx in -cells_to_check..=cells_to_check {
            for dy in -cells_to_check..=cells_to_check {
                for dz in -cells_to_check..=cells_to_check {
                    let cell = (center_cell.0 + dx, center_cell.1 + dy, center_cell.2 + dz);
                    
                    if let Some(entities) = self.grid.get(&cell) {
                        results.extend(entities.iter().copied());
                    }
                }
            }
        }
        
        results
    }
    
    /// Convert position to grid cell
    fn position_to_cell(&self, position: &Position) -> (i32, i32, i32) {
        (
            (position.x / self.cell_size).floor() as i32,
            (position.y / self.cell_size).floor() as i32,
            (position.z / self.cell_size).floor() as i32,
        )
    }
}

impl Default for SpatialQuery {
    fn default() -> Self {
        Self::new(16.0) // 16 block cells (chunk-sized)
    }
}
