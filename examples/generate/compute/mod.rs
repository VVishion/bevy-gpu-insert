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
    utils::HashSet,
};
use bevy_transfer::GpuTransfer;

use crate::generate_mesh::GenerateMesh;

use self::pipeline::GenerateMeshPipeline;

#[derive(Default)]
pub struct GenerateMeshBindGroups {
    bind_groups: Vec<(u32, BindGroup)>,
}

pub struct ChangedGenerateMeshes {
    handles: Vec<Handle<GenerateMesh>>,
}

// Maybe accumulate changed handles before the extract phase to keep extract phase shorter?
pub(crate) fn extract_generate_mesh_changes(
    mut commands: Commands,
    mut mesh_events: Extract<EventReader<AssetEvent<GenerateMesh>>>,
) {
    let mut changed = HashSet::default();

    for event in mesh_events.iter() {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                changed.insert(handle.clone_weak());
            }
            _ => {}
        }
    }

    let handles = changed.drain().collect();
    commands.insert_resource(ChangedGenerateMeshes { handles });
}

// pub(crate) fn extract_generate_mesh_transfers(
//     mut mesh_events: Extract<EventReader<AssetEvent<GenerateMesh>>>,
//     mut transfers: ResMut<Vec<Transfer<GenerateMesh, GeneratedMesh, Vertices>>>,
// ) {
//     let mut changed = HashSet::default();

//     for event in mesh_events.iter() {
//         match event {
//             AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
//                 changed.insert(handle.clone_weak());
//             }
//             _ => {}
//         }
//     }

//     let handles = changed.drain().collect();

//     handles.map()
// }

pub(crate) fn queue_generate_mesh_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<GenerateMeshPipeline>,
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
