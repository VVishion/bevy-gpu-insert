pub mod graph;
pub mod pipeline;

use bevy::{
    prelude::{AssetEvent, Commands, EventReader, Handle, Res},
    render::{
        render_asset::RenderAssets,
        render_resource::{BindGroup, BindGroupDescriptor, BindGroupEntry},
        renderer::RenderDevice,
        Extract,
    },
};

use crate::generate_mesh::GenerateMesh;

use self::pipeline::GenerateMeshPipeline;

#[derive(Default)]
pub struct GenerateMeshBindGroups {
    bind_groups: Vec<(u32, BindGroup)>,
}

pub struct ChangedGenerateMeshes {
    // mesh_handles: Vec<Handle<TerrainMesh>>,
    handles: Vec<Handle<GenerateMesh>>,
}

// Maybe accumulate changed handles before the extract phase to keep extract phase shorter?
pub(crate) fn extract_generate_mesh_changes(
    mut commands: Commands,
    mut mesh_events: Extract<EventReader<AssetEvent<GenerateMesh>>>,
    //mut image_events: Extract<EventReader<AssetEvent<Image>>>,
) {
    let mut handles = Vec::new();

    for event in mesh_events.iter() {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                handles.push(handle.clone_weak());
            }
            _ => {}
        }
    }

    commands.insert_resource(ChangedGenerateMeshes { handles });
}

pub(crate) fn queue_generate_mesh_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<GenerateMeshPipeline>,
    // query: Query<(&Terrain, &Handle<Mesh>)>,
    changed_meshes: Res<ChangedGenerateMeshes>,
    gpu_generate_meshes: Res<RenderAssets<GenerateMesh>>,
) {
    let mut bind_groups = Vec::new();

    for handle in &changed_meshes.handles {
        if let Some(gpu_generate_mesh) = gpu_generate_meshes.get(handle) {
            let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: gpu_generate_mesh.subdivisions_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: gpu_generate_mesh.buffer.as_entire_binding(),
                    },
                ],
                label: Some("generate mesh bind group"),
                layout: &pipeline.bind_group_layout,
            });

            bind_groups.push((gpu_generate_mesh.subdivisions, bind_group));
        }
    }

    commands.insert_resource(GenerateMeshBindGroups { bind_groups });
}

// only queue if changed
// pub fn queue_generate_mesh_bind_groups(
//     mut commands: Commands,
//     render_device: Res<RenderDevice>,
//     pipeline: Res<GenerateTerrainMeshPipeline>,
//     gpu_generate_meshes: Res<RenderAssets<GenerateMesh>>,
// ) {
//     let mut bind_groups = Vec::new();

//     for (_, gpu_generate_mesh) in gpu_generate_meshes.iter() {
//         let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
//             entries: &[
//                 BindGroupEntry {
//                     binding: 0,
//                     resource: gpu_generate_mesh.subdivisions_buffer.as_entire_binding(),
//                 },
//                 BindGroupEntry {
//                     binding: 1,
//                     resource: gpu_generate_mesh.buffer.as_entire_binding(),
//                 },
//             ],
//             label: Some("generate mesh bind group"),
//             layout: &pipeline.bind_group_layout,
//         });

//         bind_groups.push((gpu_generate_mesh.subdivisions, bind_group));
//     }

//     commands.insert_resource(GenerateTerrainMeshBindGroups { bind_groups });
// }
