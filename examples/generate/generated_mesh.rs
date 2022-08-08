use crate::into_render_asset::IntoRenderAsset;
use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::MeshUniform,
    prelude::{Commands, Deref, Entity, GlobalTransform, Handle, Mesh, Query, With},
    reflect::TypeUuid,
    render::{
        mesh::{GpuBufferInfo, GpuMesh, Indices},
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::{BufferInitDescriptor, BufferUsages, PrimitiveTopology},
        renderer::RenderDevice,
        Extract,
    },
};
use bevy_generate_mesh_on_gpu::FromRaw;

#[derive(TypeUuid, Clone, Deref)]
#[uuid = "2b6378c3-e473-499f-99b6-7172e6eb0d5a"]
pub struct GeneratedMesh(pub Mesh);

impl FromRaw for GeneratedMesh {
    fn from_raw(data: &[u8]) -> Self {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        let data: Vec<_> = data
            .chunks_exact(4)
            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
            .collect();

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        for chunk in data.chunks_exact(8) {
            let position = [chunk[0], chunk[1], chunk[2]];
            positions.push(position);
            println!("{position:?}");
            normals.push([chunk[3], chunk[4], chunk[5]]);
            uvs.push([chunk[6], chunk[7]]);
        }

        let subdivisions = (positions.len() as f32).sqrt() as u32 - 1;

        {
            let index = |x, y| x + y * (subdivisions + 1);

            for (x, y) in itertools::iproduct!(0..subdivisions, 0..subdivisions) {
                indices.push(index(x, y + 1));
                indices.push(index(x + 1, y));
                indices.push(index(x, y));

                indices.push(index(x, y + 1));
                indices.push(index(x + 1, y + 1));
                indices.push(index(x + 1, y));
            }
        }

        let indices = Indices::U32(indices);

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(indices));

        GeneratedMesh(mesh)
    }
}

impl IntoRenderAsset for GeneratedMesh {
    type ExtractedAsset = GeneratedMesh;
    type Into = Mesh;
    type Param = SRes<RenderDevice>;

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset_into(
        terrain_mesh: Self::ExtractedAsset,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<<Self::Into as RenderAsset>::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>>
    {
        let vertex_buffer_data = terrain_mesh.get_vertex_buffer_data();
        let vertex_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
            label: Some("generated mesh vertex buffer"),
            contents: &vertex_buffer_data,
        });

        let buffer_info = terrain_mesh.get_index_buffer_bytes().map_or(
            GpuBufferInfo::NonIndexed {
                vertex_count: terrain_mesh.count_vertices() as u32,
            },
            |data| GpuBufferInfo::Indexed {
                buffer: render_device.create_buffer_with_data(&BufferInitDescriptor {
                    usage: BufferUsages::INDEX,
                    contents: data,
                    label: Some("terrain mesh index buffer"),
                }),
                count: terrain_mesh.indices().unwrap().len() as u32,
                index_format: terrain_mesh.indices().unwrap().into(),
            },
        );

        let mesh_vertex_buffer_layout = terrain_mesh.get_mesh_vertex_buffer_layout();

        Ok(GpuMesh {
            vertex_buffer,
            buffer_info,
            primitive_topology: terrain_mesh.primitive_topology(),
            layout: mesh_vertex_buffer_layout,
        })
    }
}

pub(crate) fn extract_generated_mesh(
    mut commands: Commands,
    query: Extract<Query<(Entity, &GlobalTransform), With<Handle<GeneratedMesh>>>>,
) {
    for (entity, transform) in query.iter() {
        let transform = transform.compute_matrix();

        commands.get_or_spawn(entity).insert(MeshUniform {
            flags: 0,
            transform,
            inverse_transpose_model: transform.inverse().transpose(),
        });
    }
}
