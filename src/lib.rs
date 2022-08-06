use bevy::{
    prelude::{AddAsset, App, CoreStage, Mesh, Plugin},
    render::{
        self, render_asset::RenderAssetPlugin, render_graph::RenderGraph,
        render_resource::PrimitiveTopology, RenderApp, RenderStage,
    },
};
use compute::{
    graph::GenerateTerrainMeshNode, pipeline::GenerateTerrainMeshPipeline,
    queue_generate_mesh_bind_groups, GenerateTerrainMeshBindGroups,
};

pub mod compute;
mod from_raw;
mod generate_mesh;
mod mirror_handle;
pub mod transfer;

pub use from_raw::FromRaw;
pub use generate_mesh::GenerateMesh;
use transfer::{
    extract_transfers, prepare_transfers, queue_extract_transfers, resolve_pending_transfers,
    PrepareNextFrameTransfers,
};
pub use transfer::{GpuTransfer, Transfer, TransferDescriptor, Transferable};

impl FromRaw for Mesh {
    fn from_raw(data: &[u8]) -> Self {
        println!("{data:?}");
        Mesh::new(PrimitiveTopology::TriangleList)
    }
}

pub struct GenerateMeshPlugin;

impl Plugin for GenerateMeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<GenerateMesh>()
            .init_resource::<Vec<Transfer<GenerateMesh, Mesh>>>()
            .add_plugin(RenderAssetPlugin::<GenerateMesh>::default())
            // RenderApp is sub app to the App and is run after the App Schedule (App Stages)
            // could also be in First after marking?
            .add_system_to_stage(
                CoreStage::First,
                resolve_pending_transfers::<GenerateMesh, Mesh>,
            )
            .add_system_to_stage(
                CoreStage::Last,
                queue_extract_transfers::<GenerateMesh, Mesh>,
            );

        let (sender, receiver) = transfer::create_transfer_channels::<GenerateMesh, Mesh>();
        app.insert_resource(receiver);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(sender)
                .init_resource::<PrepareNextFrameTransfers<GenerateMesh, Mesh>>()
                .init_resource::<Vec<GpuTransfer<GenerateMesh, Mesh>>>()
                .init_resource::<GenerateTerrainMeshPipeline>()
                .init_resource::<GenerateTerrainMeshBindGroups>()
                .add_system_to_stage(
                    RenderStage::Extract,
                    extract_transfers::<GenerateMesh, Mesh>,
                )
                .add_system_to_stage(
                    RenderStage::Prepare,
                    prepare_transfers::<GenerateMesh, Mesh>,
                )
                .add_system_to_stage(RenderStage::Queue, queue_generate_mesh_bind_groups);

            let generate_terrain_mesh_node = GenerateTerrainMeshNode::new(&mut render_app.world);

            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
            render_graph.add_node(
                compute::graph::node::GENERATE_TERRAIN_MESH,
                generate_terrain_mesh_node,
            );

            render_graph
                .add_node_edge(
                    compute::graph::node::GENERATE_TERRAIN_MESH,
                    render::main_graph::node::CAMERA_DRIVER,
                )
                .unwrap();
        }
    }
}
