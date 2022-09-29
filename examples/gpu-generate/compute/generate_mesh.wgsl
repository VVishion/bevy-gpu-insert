struct Vertex {
    x: f32,
    y: f32,
    z: f32,

    nx: f32,
    ny: f32,
    nz: f32,

    u: f32,
    v: f32,
};

@group(0) @binding(0)
var<storage, read_write> vertices: array<Vertex>;

fn index(i: u32, j: u32, lj: u32) -> u32 {
    return i + j * lj;
}

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let workgroup_index = index(workgroup_id.x, workgroup_id.y, num_workgroups.y);

    let spacing = vec2<f32>(1f / f32(num_workgroups.x), 1f / f32(num_workgroups.y));

    let x = f32(workgroup_id.x) * spacing.x;
    let z = f32(workgroup_id.y) * spacing.y;
    
    vertices[workgroup_index] = Vertex(
            x - 0.5f, 0.0f, z - 0.5f,
            0f, 1f, 0f,
            x, z,
        );
};