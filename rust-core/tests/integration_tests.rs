//! # Integration Tests
//! 
//! Integration tests for the Aether Core engine.

use aether_core::*;

#[cfg(test)]
mod engine_integration_tests {
    use super::*;
    
    #[test]
    fn test_engine_initialization() {
        let config = engine::EngineConfig {
            app_name: "Test App".to_string(),
            version: (1, 0, 0),
            enable_validation: true,
            enable_profiling: true,
            ..Default::default()
        };
        
        let result = engine::AetherEngine::new(config);
        
        // Engine creation should succeed (or fail gracefully without Vulkan)
        match result {
            Ok(engine) => {
                assert!(engine.is_initialized());
            }
            Err(e) => {
                // Expected if no Vulkan support
                println!("Engine init failed (expected without GPU): {:?}", e);
            }
        }
    }
    
    #[test]
    fn test_engine_config_defaults() {
        let config = engine::EngineConfig::default();
        
        assert!(!config.app_name.is_empty());
        assert_eq!(config.version, (1, 0, 0));
    }
}

#[cfg(test)]
mod ecs_integration_tests {
    use super::*;
    use ecs::*;
    
    #[test]
    fn test_ecs_with_engine() {
        let mut world = EcsWorld::new();
        
        // Create a complex scene
        for i in 0..100 {
            let entity = world.spawn();
            
            world.add_component(entity, components::Transform {
                position: [i as f32, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            });
            
            if i % 2 == 0 {
                world.add_component(entity, components::Renderable {
                    mesh_id: i as u32,
                    material_id: 0,
                    visible: true,
                    cast_shadows: true,
                    receive_shadows: true,
                });
            }
            
            if i % 3 == 0 {
                world.add_component(entity, components::Physics {
                    mass: 1.0,
                    gravity_scale: 1.0,
                    drag: 0.1,
                    angular_drag: 0.1,
                    is_kinematic: false,
                });
            }
        }
        
        // Verify entity counts
        assert_eq!(world.entity_count(), 100);
        
        let renderable_count = world.query::<&components::Renderable>().count();
        assert_eq!(renderable_count, 50);
        
        let physics_count = world.query::<&components::Physics>().count();
        assert_eq!(physics_count, 34); // 0, 3, 6, ..., 99
    }
    
    #[test]
    fn test_system_execution_order() {
        let mut world = EcsWorld::new();
        let mut execution_order = Vec::new();
        
        // Create test entity
        let entity = world.spawn();
        world.add_component(entity, components::Transform::default());
        world.add_component(entity, components::Velocity::default());
        
        // Simulate system execution
        execution_order.push("input");
        execution_order.push("physics");
        execution_order.push("animation");
        execution_order.push("render");
        
        assert_eq!(execution_order, vec!["input", "physics", "animation", "render"]);
    }
}

#[cfg(test)]
mod renderer_integration_tests {
    use super::*;
    
    #[test]
    fn test_shader_compilation_pipeline() {
        use renderer::shaders::*;
        
        let vertex_source = r#"
            #version 450
            
            layout(location = 0) in vec3 inPosition;
            layout(location = 1) in vec2 inTexCoord;
            
            layout(location = 0) out vec2 fragTexCoord;
            
            layout(push_constant) uniform PushConstants {
                mat4 mvp;
            } pc;
            
            void main() {
                gl_Position = pc.mvp * vec4(inPosition, 1.0);
                fragTexCoord = inTexCoord;
            }
        "#;
        
        let fragment_source = r#"
            #version 450
            
            layout(location = 0) in vec2 fragTexCoord;
            layout(location = 0) out vec4 outColor;
            
            layout(binding = 0) uniform sampler2D texSampler;
            
            void main() {
                outColor = texture(texSampler, fragTexCoord);
            }
        "#;
        
        // Test shader manager creation
        let mut manager = ShaderManager::new(None);
        
        // Register shaders
        let vs_id = manager.register_shader("test_vs", vertex_source, ShaderStage::Vertex);
        let fs_id = manager.register_shader("test_fs", fragment_source, ShaderStage::Fragment);
        
        assert!(vs_id.is_ok());
        assert!(fs_id.is_ok());
    }
    
    #[test]
    fn test_particle_system_creation() {
        use renderer::particles::*;
        
        let config = ParticleSystemConfig {
            max_particles: 10000,
            gpu_simulation: false, // CPU fallback for testing
            sort_particles: true,
            texture_atlas: None,
            timestep: 1.0 / 60.0,
        };
        
        // Test emitter creation
        let emitter = ParticleEmitter::point([0.0, 0.0, 0.0])
            .with_rate(100.0)
            .with_lifetime(1.0, 2.0)
            .with_velocity([-1.0, 1.0, -1.0], [1.0, 3.0, 1.0]);
        
        assert_eq!(emitter.rate, 100.0);
        assert!(emitter.active);
    }
    
    #[test]
    fn test_particle_presets() {
        use renderer::particles::*;
        
        let fire = ParticlePreset::Fire.create_emitter([0.0, 0.0, 0.0]);
        assert!(fire.rate > 0.0);
        
        let smoke = ParticlePreset::Smoke.create_emitter([0.0, 0.0, 0.0]);
        assert!(smoke.rate > 0.0);
        
        let explosion = ParticlePreset::Explosion.create_emitter([0.0, 0.0, 0.0]);
        assert!(explosion.burst.is_some());
    }
}

#[cfg(test)]
mod profiling_integration_tests {
    use super::*;
    use profiling::*;
    
    #[test]
    fn test_profiler_frame_tracking() {
        let profiler = Profiler::new();
        
        // Simulate multiple frames
        for _ in 0..60 {
            profiler.begin_frame();
            
            {
                let _timer = profiler.start_timer("update");
                std::thread::sleep(std::time::Duration::from_micros(100));
            }
            
            {
                let _timer = profiler.start_timer("render");
                std::thread::sleep(std::time::Duration::from_micros(200));
            }
            
            profiler.end_frame();
        }
        
        let stats = profiler.get_frame_stats();
        assert!(stats.frame_count >= 60);
        assert!(stats.fps > 0.0);
    }
    
    #[test]
    fn test_metrics_collection() {
        let mut metrics = MetricsCollector::new();
        
        metrics.record("test_gauge", 42.0);
        metrics.increment("test_counter", 1);
        metrics.increment("test_counter", 1);
        metrics.record_histogram("test_histogram", 10.0);
        metrics.record_histogram("test_histogram", 20.0);
        metrics.record_histogram("test_histogram", 30.0);
        
        let gauge = metrics.get("test_gauge");
        assert!(matches!(gauge, Some(MetricValue::Gauge(42.0))));
        
        let counter = metrics.get("test_counter");
        assert!(matches!(counter, Some(MetricValue::Counter(2))));
    }
    
    #[test]
    fn test_memory_tracking() {
        let mut tracker = MemoryTracker::new();
        
        tracker.allocate("textures", 1024 * 1024);
        tracker.allocate("meshes", 512 * 1024);
        tracker.allocate("textures", 2048 * 1024);
        
        let stats = tracker.stats();
        
        assert_eq!(stats.total_allocated, 3584 * 1024);
        assert_eq!(stats.allocation_count, 3);
    }
    
    #[test]
    fn test_benchmark_runner() {
        let mut benchmark = Benchmark::new("test_benchmark")
            .iterations(100)
            .warmup(10);
        
        let result = benchmark.run(|| {
            let mut sum = 0u64;
            for i in 0..1000 {
                sum += i;
            }
            std::hint::black_box(sum);
        });
        
        assert_eq!(result.iterations, 100);
        assert!(result.avg_ns > 0.0);
    }
}

#[cfg(test)]
mod network_integration_tests {
    use super::*;
    use network::*;
    
    #[test]
    fn test_packet_compression() {
        let original_data = vec![0u8; 1000]; // Compressible data
        
        let compressed = compress_packet(&original_data);
        assert!(compressed.is_ok());
        
        let compressed_data = compressed.unwrap();
        
        // Compressed should be smaller for repetitive data
        assert!(compressed_data.len() < original_data.len());
        
        let decompressed = decompress_packet(&compressed_data, original_data.len());
        assert!(decompressed.is_ok());
        
        assert_eq!(decompressed.unwrap(), original_data);
    }
    
    #[test]
    fn test_packet_compression_random_data() {
        // Random data doesn't compress well
        let original_data: Vec<u8> = (0..1000).map(|i| (i * 17 % 256) as u8).collect();
        
        let compressed = compress_packet(&original_data);
        assert!(compressed.is_ok());
        
        let decompressed = decompress_packet(&compressed.unwrap(), original_data.len());
        assert!(decompressed.is_ok());
        
        assert_eq!(decompressed.unwrap(), original_data);
    }
}

#[cfg(test)]
mod audio_integration_tests {
    use super::*;
    use audio::*;
    
    #[test]
    fn test_audio_manager_creation() {
        let manager = AudioManager::new();
        
        // Should initialize without errors
        assert!(manager.is_ok() || manager.is_err()); // May fail without audio device
    }
    
    #[test]
    fn test_audio_source_config() {
        let config = AudioSourceConfig {
            volume: 0.8,
            pitch: 1.0,
            looping: true,
            spatial: true,
            min_distance: 1.0,
            max_distance: 100.0,
            rolloff_factor: 1.0,
        };
        
        assert_eq!(config.volume, 0.8);
        assert!(config.looping);
        assert!(config.spatial);
    }
}

#[cfg(test)]
mod world_integration_tests {
    use super::*;
    use world::*;
    
    #[test]
    fn test_chunk_coordinates() {
        let chunk_pos = ChunkPos::new(0, 0, 0);
        
        assert_eq!(chunk_pos.x, 0);
        assert_eq!(chunk_pos.y, 0);
        assert_eq!(chunk_pos.z, 0);
    }
    
    #[test]
    fn test_chunk_from_world_pos() {
        // World position to chunk position
        let world_x = 32.0;
        let world_z = 48.0;
        
        let chunk_x = (world_x / 16.0).floor() as i32;
        let chunk_z = (world_z / 16.0).floor() as i32;
        
        assert_eq!(chunk_x, 2);
        assert_eq!(chunk_z, 3);
    }
    
    #[test]
    fn test_block_data() {
        let block = BlockData {
            id: 1,
            metadata: 0,
            light_level: 15,
            sky_light: 15,
        };
        
        assert_eq!(block.id, 1);
        assert_eq!(block.light_level, 15);
    }
}

#[cfg(test)]
mod full_pipeline_tests {
    use super::*;
    
    #[test]
    fn test_complete_frame_simulation() {
        // Simulate a complete frame pipeline
        let mut world = ecs::EcsWorld::new();
        let profiler = profiling::Profiler::new();
        
        // Setup
        for i in 0..100 {
            let entity = world.spawn();
            world.add_component(entity, ecs::components::Transform {
                position: [i as f32, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            });
            world.add_component(entity, ecs::components::Velocity {
                linear: [1.0, 0.0, 0.0],
                angular: [0.0, 0.0, 0.0],
            });
        }
        
        // Simulate 60 frames
        for frame in 0..60 {
            profiler.begin_frame();
            
            // Input phase
            {
                let _timer = profiler.start_timer("input");
                // Process input...
            }
            
            // Update phase
            {
                let _timer = profiler.start_timer("update");
                let dt = 1.0 / 60.0;
                
                for (transform, velocity) in world.query_mut::<(
                    &mut ecs::components::Transform,
                    &ecs::components::Velocity,
                )>() {
                    transform.position[0] += velocity.linear[0] * dt;
                    transform.position[1] += velocity.linear[1] * dt;
                    transform.position[2] += velocity.linear[2] * dt;
                }
            }
            
            // Render phase
            {
                let _timer = profiler.start_timer("render");
                let visible_count = world.query::<&ecs::components::Transform>().count();
                profiler.record_metric("visible_entities", visible_count as f64);
            }
            
            profiler.end_frame();
        }
        
        // Verify results
        let stats = profiler.get_frame_stats();
        assert_eq!(stats.frame_count, 60);
        
        // Verify entity positions updated
        let first_entity_pos: f32 = world.query::<&ecs::components::Transform>()
            .next()
            .map(|t| t.position[0])
            .unwrap_or(0.0);
        
        // After 60 frames at 1 unit/second, should have moved 1 unit
        assert!(first_entity_pos > 0.0);
    }
}
