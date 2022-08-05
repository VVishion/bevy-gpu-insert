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

@group(0) @binding(1)
var<storage, read_write> vertex_buffer: VertexBuffer;


@compute @workgroup_size(4, 4, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let divisions = 4u;

    let x = invocation_id.x;
    let y = invocation_id.y;

    let i = x + y * 4u;

    let spacing = 1f / (f32(divisions) - 1f);

    let pos = Position(f32(x) * spacing, f32(y) * spacing);
    
    vertex_buffer.vertices[i] = Vertex(
            pos.x - 0.5f, 0.0f, pos.y - 0.5f,
            0f, 1f, 0f,
            pos.x, pos.y,
        );
};