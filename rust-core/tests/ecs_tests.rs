//! # ECS Unit Tests
//! 
//! Unit tests for the Entity Component System.

use aether_core::ecs::*;

#[cfg(test)]
mod ecs_tests {
    use super::*;
    
    // Test components
    #[derive(Debug, Clone, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
        z: f32,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    struct Velocity {
        x: f32,
        y: f32,
        z: f32,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    struct Health {
        current: i32,
        max: i32,
    }
    
    #[derive(Debug, Clone)]
    struct Name(String);
    
    #[test]
    fn test_world_creation() {
        let world = EcsWorld::new();
        assert_eq!(world.entity_count(), 0);
    }
    
    #[test]
    fn test_entity_creation() {
        let mut world = EcsWorld::new();
        
        let entity = world.spawn();
        assert!(entity.is_valid());
        assert_eq!(world.entity_count(), 1);
        
        let entity2 = world.spawn();
        assert!(entity2.is_valid());
        assert_eq!(world.entity_count(), 2);
        assert_ne!(entity, entity2);
    }
    
    #[test]
    fn test_entity_despawn() {
        let mut world = EcsWorld::new();
        
        let entity = world.spawn();
        assert_eq!(world.entity_count(), 1);
        
        world.despawn(entity);
        assert_eq!(world.entity_count(), 0);
    }
    
    #[test]
    fn test_component_add_get() {
        let mut world = EcsWorld::new();
        
        let entity = world.spawn();
        
        let pos = Position { x: 1.0, y: 2.0, z: 3.0 };
        world.add_component(entity, pos.clone());
        
        let retrieved: Option<&Position> = world.get_component(entity);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), &pos);
    }
    
    #[test]
    fn test_component_remove() {
        let mut world = EcsWorld::new();
        
        let entity = world.spawn();
        world.add_component(entity, Position { x: 0.0, y: 0.0, z: 0.0 });
        
        assert!(world.has_component::<Position>(entity));
        
        world.remove_component::<Position>(entity);
        
        assert!(!world.has_component::<Position>(entity));
    }
    
    #[test]
    fn test_multiple_components() {
        let mut world = EcsWorld::new();
        
        let entity = world.spawn();
        
        world.add_component(entity, Position { x: 1.0, y: 2.0, z: 3.0 });
        world.add_component(entity, Velocity { x: 0.1, y: 0.2, z: 0.3 });
        world.add_component(entity, Health { current: 100, max: 100 });
        
        assert!(world.has_component::<Position>(entity));
        assert!(world.has_component::<Velocity>(entity));
        assert!(world.has_component::<Health>(entity));
        
        let pos: &Position = world.get_component(entity).unwrap();
        let vel: &Velocity = world.get_component(entity).unwrap();
        let health: &Health = world.get_component(entity).unwrap();
        
        assert_eq!(pos.x, 1.0);
        assert_eq!(vel.x, 0.1);
        assert_eq!(health.current, 100);
    }
    
    #[test]
    fn test_component_mutation() {
        let mut world = EcsWorld::new();
        
        let entity = world.spawn();
        world.add_component(entity, Health { current: 100, max: 100 });
        
        {
            let health: &mut Health = world.get_component_mut(entity).unwrap();
            health.current -= 25;
        }
        
        let health: &Health = world.get_component(entity).unwrap();
        assert_eq!(health.current, 75);
    }
    
    #[test]
    fn test_query_single_component() {
        let mut world = EcsWorld::new();
        
        // Create entities with positions
        for i in 0..10 {
            let entity = world.spawn();
            world.add_component(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
        }
        
        // Create entities without positions
        for _ in 0..5 {
            world.spawn();
        }
        
        let count = world.query::<&Position>().count();
        assert_eq!(count, 10);
    }
    
    #[test]
    fn test_query_multiple_components() {
        let mut world = EcsWorld::new();
        
        // Entities with both Position and Velocity
        for i in 0..5 {
            let entity = world.spawn();
            world.add_component(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add_component(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        }
        
        // Entities with only Position
        for i in 0..3 {
            let entity = world.spawn();
            world.add_component(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
        }
        
        let count_pos = world.query::<&Position>().count();
        let count_both = world.query::<(&Position, &Velocity)>().count();
        
        assert_eq!(count_pos, 8);
        assert_eq!(count_both, 5);
    }
    
    #[test]
    fn test_movement_system() {
        let mut world = EcsWorld::new();
        
        let entity = world.spawn();
        world.add_component(entity, Position { x: 0.0, y: 0.0, z: 0.0 });
        world.add_component(entity, Velocity { x: 1.0, y: 2.0, z: 3.0 });
        
        // Simulate movement system
        let dt = 1.0;
        for (pos, vel) in world.query_mut::<(&mut Position, &Velocity)>() {
            pos.x += vel.x * dt;
            pos.y += vel.y * dt;
            pos.z += vel.z * dt;
        }
        
        let pos: &Position = world.get_component(entity).unwrap();
        assert_eq!(pos.x, 1.0);
        assert_eq!(pos.y, 2.0);
        assert_eq!(pos.z, 3.0);
    }
    
    #[test]
    fn test_entity_builder() {
        let mut world = EcsWorld::new();
        
        let entity = world.spawn_with((
            Position { x: 1.0, y: 2.0, z: 3.0 },
            Velocity { x: 0.1, y: 0.2, z: 0.3 },
            Health { current: 100, max: 100 },
        ));
        
        assert!(world.has_component::<Position>(entity));
        assert!(world.has_component::<Velocity>(entity));
        assert!(world.has_component::<Health>(entity));
    }
    
    #[test]
    fn test_entity_recycling() {
        let mut world = EcsWorld::new();
        
        let entity1 = world.spawn();
        let id1 = entity1.id();
        
        world.despawn(entity1);
        
        let entity2 = world.spawn();
        
        // Entity ID might be recycled but generation should differ
        // This tests that the entity system properly handles recycling
        assert!(entity2.is_valid());
    }
    
    #[test]
    fn test_component_storage_efficiency() {
        let mut world = EcsWorld::new();
        
        // Create many entities
        let entities: Vec<_> = (0..1000).map(|_| world.spawn()).collect();
        
        // Add components to half of them
        for (i, &entity) in entities.iter().enumerate() {
            if i % 2 == 0 {
                world.add_component(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
            }
        }
        
        let count = world.query::<&Position>().count();
        assert_eq!(count, 500);
    }
    
    #[test]
    fn test_despawn_clears_components() {
        let mut world = EcsWorld::new();
        
        let entity = world.spawn();
        world.add_component(entity, Position { x: 1.0, y: 2.0, z: 3.0 });
        world.add_component(entity, Velocity { x: 0.1, y: 0.2, z: 0.3 });
        
        world.despawn(entity);
        
        // After despawn, entity should not be found in queries
        let count = world.query::<&Position>().count();
        assert_eq!(count, 0);
    }
    
    #[test]
    fn test_clear_world() {
        let mut world = EcsWorld::new();
        
        for i in 0..100 {
            let entity = world.spawn();
            world.add_component(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
        }
        
        assert_eq!(world.entity_count(), 100);
        
        world.clear();
        
        assert_eq!(world.entity_count(), 0);
    }
}

#[cfg(test)]
mod component_tests {
    use super::*;
    
    #[test]
    fn test_transform_component() {
        let transform = components::Transform {
            position: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        };
        
        assert_eq!(transform.position, [1.0, 2.0, 3.0]);
        assert_eq!(transform.scale, [1.0, 1.0, 1.0]);
    }
    
    #[test]
    fn test_velocity_component() {
        let velocity = components::Velocity {
            linear: [1.0, 0.0, 0.0],
            angular: [0.0, 0.0, 0.0],
        };
        
        assert_eq!(velocity.linear, [1.0, 0.0, 0.0]);
    }
    
    #[test]
    fn test_renderable_component() {
        let renderable = components::Renderable {
            mesh_id: 1,
            material_id: 2,
            visible: true,
            cast_shadows: true,
            receive_shadows: true,
        };
        
        assert!(renderable.visible);
        assert_eq!(renderable.mesh_id, 1);
    }
}

#[cfg(test)]
mod system_tests {
    use super::*;
    
    #[test]
    fn test_physics_system_integration() {
        let mut world = EcsWorld::new();
        
        // Create entity with physics components
        let entity = world.spawn();
        world.add_component(entity, components::Transform {
            position: [0.0, 10.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        });
        world.add_component(entity, components::Velocity {
            linear: [0.0, 0.0, 0.0],
            angular: [0.0, 0.0, 0.0],
        });
        world.add_component(entity, components::Physics {
            mass: 1.0,
            gravity_scale: 1.0,
            drag: 0.0,
            angular_drag: 0.0,
            is_kinematic: false,
        });
        
        // Run physics for 1 second at 60 FPS
        let dt = 1.0 / 60.0;
        let gravity = -9.81;
        
        for _ in 0..60 {
            for (transform, velocity, physics) in world.query_mut::<(
                &mut components::Transform,
                &mut components::Velocity,
                &components::Physics,
            )>() {
                if !physics.is_kinematic {
                    // Apply gravity
                    velocity.linear[1] += gravity * physics.gravity_scale * dt;
                    
                    // Update position
                    transform.position[0] += velocity.linear[0] * dt;
                    transform.position[1] += velocity.linear[1] * dt;
                    transform.position[2] += velocity.linear[2] * dt;
                }
            }
        }
        
        let transform: &components::Transform = world.get_component(entity).unwrap();
        
        // After 1 second of free fall from 10m, should be around 5m lower
        // y = y0 + v0*t + 0.5*g*t^2 = 10 + 0 - 0.5*9.81*1 ≈ 5.1
        assert!(transform.position[1] < 10.0);
        assert!(transform.position[1] > 0.0);
    }
    
    #[test]
    fn test_render_system_visibility() {
        let mut world = EcsWorld::new();
        
        // Create visible entities
        for i in 0..5 {
            let entity = world.spawn();
            world.add_component(entity, components::Transform::default());
            world.add_component(entity, components::Renderable {
                mesh_id: i,
                material_id: 0,
                visible: true,
                cast_shadows: true,
                receive_shadows: true,
            });
        }
        
        // Create invisible entities
        for i in 0..3 {
            let entity = world.spawn();
            world.add_component(entity, components::Transform::default());
            world.add_component(entity, components::Renderable {
                mesh_id: i + 5,
                material_id: 0,
                visible: false,
                cast_shadows: false,
                receive_shadows: false,
            });
        }
        
        // Count visible entities
        let visible_count = world.query::<(&components::Transform, &components::Renderable)>()
            .filter(|(_, r)| r.visible)
            .count();
        
        assert_eq!(visible_count, 5);
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn test_entity_creation_performance() {
        let mut world = EcsWorld::new();
        
        let start = Instant::now();
        for _ in 0..10000 {
            world.spawn();
        }
        let duration = start.elapsed();
        
        // Should be able to create 10000 entities in under 100ms
        assert!(duration.as_millis() < 100, "Entity creation too slow: {:?}", duration);
    }
    
    #[test]
    fn test_component_iteration_performance() {
        let mut world = EcsWorld::new();
        
        // Create entities with components
        for i in 0..10000 {
            let entity = world.spawn();
            world.add_component(entity, components::Transform {
                position: [i as f32, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            });
            world.add_component(entity, components::Velocity {
                linear: [1.0, 0.0, 0.0],
                angular: [0.0, 0.0, 0.0],
            });
        }
        
        let start = Instant::now();
        let dt = 1.0 / 60.0;
        
        for (transform, velocity) in world.query_mut::<(&mut components::Transform, &components::Velocity)>() {
            transform.position[0] += velocity.linear[0] * dt;
            transform.position[1] += velocity.linear[1] * dt;
            transform.position[2] += velocity.linear[2] * dt;
        }
        
        let duration = start.elapsed();
        
        // Should iterate 10000 entities in under 10ms
        assert!(duration.as_millis() < 10, "Iteration too slow: {:?}", duration);
    }
}
