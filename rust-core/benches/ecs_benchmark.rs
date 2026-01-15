//! # ECS Benchmarks
//! 
//! Performance benchmarks for the Entity Component System.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use aether_core::ecs::*;

// Test components
#[derive(Debug, Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Clone, Copy)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Clone, Copy)]
struct Rotation {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[derive(Debug, Clone, Copy)]
struct Health {
    current: i32,
    max: i32,
}

fn bench_entity_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_creation");
    
    for count in [100, 1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let mut world = EcsWorld::new();
                for _ in 0..count {
                    black_box(world.spawn());
                }
            });
        });
    }
    
    group.finish();
}

fn bench_entity_creation_with_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_creation_with_components");
    
    for count in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let mut world = EcsWorld::new();
                for i in 0..count {
                    let entity = world.spawn();
                    world.add_component(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
                    world.add_component(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
                }
            });
        });
    }
    
    group.finish();
}

fn bench_component_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_iteration");
    
    for count in [1000, 10000, 100000].iter() {
        // Setup world with entities
        let mut world = EcsWorld::new();
        for i in 0..*count {
            let entity = world.spawn();
            world.add_component(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add_component(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        }
        
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                let dt = 1.0 / 60.0;
                for (pos, vel) in world.query_mut::<(&mut Position, &Velocity)>() {
                    pos.x += vel.x * dt;
                    pos.y += vel.y * dt;
                    pos.z += vel.z * dt;
                }
            });
        });
    }
    
    group.finish();
}

fn bench_component_iteration_multiple(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_iteration_multiple");
    
    for count in [1000, 10000, 50000].iter() {
        // Setup world with entities having multiple components
        let mut world = EcsWorld::new();
        for i in 0..*count {
            let entity = world.spawn();
            world.add_component(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add_component(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            world.add_component(entity, Rotation { x: 0.0, y: 0.0, z: 0.0, w: 1.0 });
            world.add_component(entity, Health { current: 100, max: 100 });
        }
        
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                for (pos, vel, rot, health) in world.query_mut::<(
                    &mut Position,
                    &Velocity,
                    &mut Rotation,
                    &Health,
                )>() {
                    pos.x += vel.x;
                    rot.w = rot.w.min(1.0);
                    black_box(health.current);
                }
            });
        });
    }
    
    group.finish();
}

fn bench_entity_despawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_despawn");
    
    for count in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter_batched(
                || {
                    let mut world = EcsWorld::new();
                    let entities: Vec<_> = (0..count).map(|_| world.spawn()).collect();
                    (world, entities)
                },
                |(mut world, entities)| {
                    for entity in entities {
                        world.despawn(entity);
                    }
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    
    group.finish();
}

fn bench_component_add_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_add_remove");
    
    for count in [100, 1000, 5000].iter() {
        // Setup world with entities
        let mut world = EcsWorld::new();
        let entities: Vec<_> = (0..*count).map(|_| world.spawn()).collect();
        
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::new("add", count), count, |b, _| {
            b.iter(|| {
                for &entity in &entities {
                    world.add_component(entity, Position { x: 0.0, y: 0.0, z: 0.0 });
                }
                for &entity in &entities {
                    world.remove_component::<Position>(entity);
                }
            });
        });
    }
    
    group.finish();
}

fn bench_query_sparse(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_sparse");
    
    // Create world where only some entities have the queried component
    let mut world = EcsWorld::new();
    for i in 0..10000 {
        let entity = world.spawn();
        world.add_component(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
        
        // Only 10% have velocity
        if i % 10 == 0 {
            world.add_component(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        }
    }
    
    group.bench_function("sparse_10_percent", |b| {
        b.iter(|| {
            let count = world.query::<(&Position, &Velocity)>().count();
            black_box(count);
        });
    });
    
    group.finish();
}

fn bench_parallel_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_iteration");
    
    for count in [10000, 50000, 100000].iter() {
        let mut world = EcsWorld::new();
        for i in 0..*count {
            let entity = world.spawn();
            world.add_component(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add_component(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        }
        
        group.throughput(Throughput::Elements(*count as u64));
        
        // Sequential
        group.bench_with_input(BenchmarkId::new("sequential", count), count, |b, _| {
            b.iter(|| {
                for (pos, vel) in world.query_mut::<(&mut Position, &Velocity)>() {
                    pos.x += vel.x;
                    pos.y += vel.y;
                    pos.z += vel.z;
                }
            });
        });
        
        // Note: Parallel benchmark would require parallel query support
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_entity_creation,
    bench_entity_creation_with_components,
    bench_component_iteration,
    bench_component_iteration_multiple,
    bench_entity_despawn,
    bench_component_add_remove,
    bench_query_sparse,
    bench_parallel_iteration,
);

criterion_main!(benches);
