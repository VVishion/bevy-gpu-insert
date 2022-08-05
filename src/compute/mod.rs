pub mod graph;
pub mod pipeline;

use bevy::{
    asset::Asset,
    prelude::{Commands, Res},
    render::{
        render_asset::RenderAssets,
        render_resource::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource},
        renderer::RenderDevice,
    },
};

use crate::generate_mesh::GenerateMesh;

use self::pipeline::GenerateTerrainMeshPipeline;

#[derive(Default)]
pub struct GenerateTerrainMeshBindGroups {
    bind_groups: Vec<BindGroup>,
}

// only queue if changed
pub fn queue_generate_mesh_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<GenerateTerrainMeshPipeline>,
    gpu_generate_meshes: Res<RenderAssets<GenerateMesh>>,
) {
    let mut bind_groups = Vec::new();

    for (_, gpu_generate_mesh) in gpu_generate_meshes.iter() {
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 1,
                resource: gpu_generate_mesh.buffer.as_entire_binding(),
            }],
            label: Some("generate mesh bind group"),
            layout: &pipeline.bind_group_layout,
        });

        bind_groups.push(bind_group);
    }

    commands.insert_resource(GenerateTerrainMeshBindGroups { bind_groups });
}
