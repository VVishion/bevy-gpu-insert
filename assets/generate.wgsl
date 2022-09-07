struct Position {
    x: f32,
    y: f32,
};

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

struct VertexBuffer {
    vertices: array<Vertex>,
};

@group(0) @binding(0)
var<uniform> subdivisions: u32;

@group(0) @binding(1)
var<storage, read_write> vertex_buffer: VertexBuffer;


@compute @workgroup_size(1, 1, 1)
fn main(@builtin(workgroup_id) workgroup_id: vec3<u32>) {
    let x = workgroup_id.x;
    let y = workgroup_id.y;

    let i = x + y * (subdivisions + 1u);

    let spacing = 1f / (f32(subdivisions));

    let pos = Position(f32(x) * spacing, f32(y) * spacing);
    
    vertex_buffer.vertices[i] = Vertex(
            pos.x - 0.5f, 0.0f, pos.y - 0.5f,
            0f, 1f, 0f,
            pos.x, pos.y,
        );
};