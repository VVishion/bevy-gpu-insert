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
var<uniform> subdivisions: u32;

@group(0) @binding(1)
var<storage, read_write> vertices: array<Vertex>;


@compute @workgroup_size(1, 1, 1)
fn main(@builtin(workgroup_id) workgroup_id: vec3<u32>) {
    let k = workgroup_id.x;
    let l = workgroup_id.y;

    // 2 dimensional to 1 dimensional index.
    let i = k + l * (subdivisions + 1u);

    let spacing = 1f / (f32(subdivisions));

    let x = f32(k) * spacing;
    let z = f32(l) * spacing;
    
    vertices[i] = Vertex(
            x - 0.5f, 0.0f, z - 0.5f,
            0f, 1f, 0f,
            x, z,
        );
};