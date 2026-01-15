//! # Render Benchmarks
//! 
//! Performance benchmarks for rendering systems.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};

// Simulated vertex data
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

// Simulated mesh data
struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Mesh {
    fn cube() -> Self {
        // Simplified cube mesh
        let vertices = vec![
            // Front face
            Vertex { position: [-1.0, -1.0, 1.0], normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0] },
            Vertex { position: [1.0, -1.0, 1.0], normal: [0.0, 0.0, 1.0], uv: [1.0, 0.0] },
            Vertex { position: [1.0, 1.0, 1.0], normal: [0.0, 0.0, 1.0], uv: [1.0, 1.0] },
            Vertex { position: [-1.0, 1.0, 1.0], normal: [0.0, 0.0, 1.0], uv: [0.0, 1.0] },
            // Back face
            Vertex { position: [-1.0, -1.0, -1.0], normal: [0.0, 0.0, -1.0], uv: [0.0, 0.0] },
            Vertex { position: [1.0, -1.0, -1.0], normal: [0.0, 0.0, -1.0], uv: [1.0, 0.0] },
            Vertex { position: [1.0, 1.0, -1.0], normal: [0.0, 0.0, -1.0], uv: [1.0, 1.0] },
            Vertex { position: [-1.0, 1.0, -1.0], normal: [0.0, 0.0, -1.0], uv: [0.0, 1.0] },
        ];
        
        let indices = vec![
            0, 1, 2, 2, 3, 0, // Front
            4, 5, 6, 6, 7, 4, // Back
            0, 4, 7, 7, 3, 0, // Left
            1, 5, 6, 6, 2, 1, // Right
            3, 2, 6, 6, 7, 3, // Top
            0, 1, 5, 5, 4, 0, // Bottom
        ];
        
        Self { vertices, indices }
    }
}

fn bench_frustum_culling(c: &mut Criterion) {
    let mut group = c.benchmark_group("frustum_culling");
    
    // Simulated frustum planes
    let frustum_planes: [[f32; 4]; 6] = [
        [1.0, 0.0, 0.0, 100.0],  // Left
        [-1.0, 0.0, 0.0, 100.0], // Right
        [0.0, 1.0, 0.0, 100.0],  // Bottom
        [0.0, -1.0, 0.0, 100.0], // Top
        [0.0, 0.0, 1.0, 1.0],    // Near
        [0.0, 0.0, -1.0, 1000.0], // Far
    ];
    
    for count in [1000, 10000, 100000].iter() {
        // Generate random bounding boxes
        let bboxes: Vec<([f32; 3], [f32; 3])> = (0..*count)
            .map(|i| {
                let x = (i % 100) as f32 * 10.0 - 500.0;
                let y = ((i / 100) % 100) as f32 * 10.0 - 500.0;
                let z = (i / 10000) as f32 * 10.0;
                ([x - 1.0, y - 1.0, z - 1.0], [x + 1.0, y + 1.0, z + 1.0])
            })
            .collect();
        
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                let mut visible = 0;
                for (min, max) in &bboxes {
                    if is_box_in_frustum(&frustum_planes, min, max) {
                        visible += 1;
                    }
                }
                black_box(visible)
            });
        });
    }
    
    group.finish();
}

fn is_box_in_frustum(planes: &[[f32; 4]; 6], min: &[f32; 3], max: &[f32; 3]) -> bool {
    for plane in planes {
        let px = if plane[0] > 0.0 { max[0] } else { min[0] };
        let py = if plane[1] > 0.0 { max[1] } else { min[1] };
        let pz = if plane[2] > 0.0 { max[2] } else { min[2] };
        
        let dist = plane[0] * px + plane[1] * py + plane[2] * pz + plane[3];
        if dist < 0.0 {
            return false;
        }
    }
    true
}

fn bench_matrix_multiplication(c: &mut Criterion) {
    let mut group = c.benchmark_group("matrix_multiplication");
    
    // 4x4 matrices
    let mat_a: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [1.0, 2.0, 3.0, 1.0],
    ];
    
    let mat_b: [[f32; 4]; 4] = [
        [0.707, 0.0, 0.707, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [-0.707, 0.0, 0.707, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    
    for count in [1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.iter(|| {
                let mut result = mat_a;
                for _ in 0..count {
                    result = multiply_matrices(&result, &mat_b);
                }
                black_box(result)
            });
        });
    }
    
    group.finish();
}

fn multiply_matrices(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0f32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    result
}

fn bench_vertex_transform(c: &mut Criterion) {
    let mut group = c.benchmark_group("vertex_transform");
    
    let transform: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [10.0, 20.0, 30.0, 1.0],
    ];
    
    for count in [1000, 10000, 100000].iter() {
        let vertices: Vec<[f32; 3]> = (0..*count)
            .map(|i| [i as f32, (i * 2) as f32, (i * 3) as f32])
            .collect();
        
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                let transformed: Vec<[f32; 3]> = vertices.iter()
                    .map(|v| transform_vertex(v, &transform))
                    .collect();
                black_box(transformed)
            });
        });
    }
    
    group.finish();
}

fn transform_vertex(v: &[f32; 3], m: &[[f32; 4]; 4]) -> [f32; 3] {
    [
        v[0] * m[0][0] + v[1] * m[1][0] + v[2] * m[2][0] + m[3][0],
        v[0] * m[0][1] + v[1] * m[1][1] + v[2] * m[2][1] + m[3][1],
        v[0] * m[0][2] + v[1] * m[1][2] + v[2] * m[2][2] + m[3][2],
    ]
}

fn bench_mesh_batching(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_batching");
    
    // Create multiple mesh instances
    let base_mesh = Mesh::cube();
    
    for instance_count in [100, 1000, 5000].iter() {
        let instances: Vec<[[f32; 4]; 4]> = (0..*instance_count)
            .map(|i| {
                let x = (i % 10) as f32 * 5.0;
                let y = ((i / 10) % 10) as f32 * 5.0;
                let z = (i / 100) as f32 * 5.0;
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [x, y, z, 1.0],
                ]
            })
            .collect();
        
        group.throughput(Throughput::Elements(*instance_count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(instance_count), instance_count, |b, _| {
            b.iter(|| {
                // Simulate batching vertices for all instances
                let total_vertices = base_mesh.vertices.len() * instances.len();
                let mut batched = Vec::with_capacity(total_vertices);
                
                for transform in &instances {
                    for vertex in &base_mesh.vertices {
                        let pos = transform_vertex(&vertex.position, transform);
                        batched.push(Vertex {
                            position: pos,
                            normal: vertex.normal,
                            uv: vertex.uv,
                        });
                    }
                }
                
                black_box(batched)
            });
        });
    }
    
    group.finish();
}

fn bench_depth_sorting(c: &mut Criterion) {
    let mut group = c.benchmark_group("depth_sorting");
    
    for count in [100, 1000, 10000].iter() {
        let objects: Vec<(u32, f32)> = (0..*count)
            .map(|i| (i as u32, (i as f32 * 0.1).sin() * 100.0))
            .collect();
        
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter_batched(
                || objects.clone(),
                |mut objs| {
                    objs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                    black_box(objs)
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    
    group.finish();
}

fn bench_occlusion_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("occlusion_query");
    
    // Simulate hierarchical occlusion with bounding volume hierarchy
    for node_count in [100, 1000, 5000].iter() {
        let nodes: Vec<([f32; 3], [f32; 3], bool)> = (0..*node_count)
            .map(|i| {
                let x = (i % 10) as f32 * 10.0;
                let y = ((i / 10) % 10) as f32 * 10.0;
                let z = (i / 100) as f32 * 10.0;
                ([x, y, z], [x + 5.0, y + 5.0, z + 5.0], i % 3 != 0)
            })
            .collect();
        
        group.throughput(Throughput::Elements(*node_count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(node_count), node_count, |b, _| {
            b.iter(|| {
                let visible: Vec<_> = nodes.iter()
                    .filter(|(_, _, occluded)| !occluded)
                    .collect();
                black_box(visible.len())
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_frustum_culling,
    bench_matrix_multiplication,
    bench_vertex_transform,
    bench_mesh_batching,
    bench_depth_sorting,
    bench_occlusion_query,
);

criterion_main!(benches);
