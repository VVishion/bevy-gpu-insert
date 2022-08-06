use bevy::{
    prelude::{Entity, Mesh, QueryState, World},
    render::{
        render_graph,
        render_resource::{CachedPipelineState, ComputePassDescriptor, PipelineCache},
        renderer::{RenderContext, RenderDevice, RenderQueue},
    },
};
use pollster::FutureExt;
use wgpu::CommandEncoderDescriptor;

use super::{pipeline::GenerateTerrainMeshPipeline, GenerateTerrainMeshBindGroups};
use crate::{
    generate_mesh::GenerateMesh,
    transfer::{PreparedTransfers, TransferSender},
    FromRaw,
};

pub mod node {
    pub const GENERATE_TERRAIN_MESH: &str = "generate_terrain_mesh";
}

enum ComputePipelineState {
    Loading,
    Ready,
}

pub(crate) struct GenerateTerrainMeshNode {
    state: ComputePipelineState,
}

impl GenerateTerrainMeshNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            state: ComputePipelineState::Loading,
        }
    }
}

impl render_graph::Node for GenerateTerrainMeshNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<GenerateTerrainMeshPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            ComputePipelineState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.pipeline)
                {
                    self.state = ComputePipelineState::Ready
                }
            }
            ComputePipelineState::Ready => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        //for (entity, terrain) in self.query.iter_manual(world) {}

        let render_device = world.resource::<RenderDevice>();
        let prepared_transfers = world.resource::<PreparedTransfers<GenerateMesh, Mesh>>();
        let transfer_sender = world.resource::<TransferSender<GenerateMesh, Mesh>>();

        let GenerateTerrainMeshBindGroups { bind_groups } =
            world.resource::<GenerateTerrainMeshBindGroups>();

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<GenerateTerrainMeshPipeline>();

        let mut encoder = render_context
            .render_device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());

            match self.state {
                ComputePipelineState::Loading => {}
                ComputePipelineState::Ready => {
                    let pipeline = pipeline_cache
                        .get_compute_pipeline(pipeline.pipeline)
                        .unwrap();
                    pass.set_pipeline(pipeline);

                    for bind_group in bind_groups.iter() {
                        pass.set_bind_group(0, bind_group, &[]);
                        pass.dispatch_workgroups(1, 1, 1);
                    }
                }
            }
        }

        for transfer in prepared_transfers.prepared.iter() {
            encoder.copy_buffer_to_buffer(
                &transfer.source,
                transfer.source_offset,
                &transfer.destination,
                transfer.destination_offset,
                transfer.size,
            );
        }

        let render_queue = world.get_resource::<RenderQueue>().unwrap();
        render_queue.submit(std::iter::once(encoder.finish()));

        for transfer in prepared_transfers.prepared.iter() {
            // transfer_sender
            //     .try_send(transfer.destination.clone())
            //     .unwrap();

            let buffer = transfer.destination.clone();

            async {
                let buffer_slice = buffer.slice(..);

                let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
                buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                    result.unwrap();
                    tx.send(()).unwrap();
                });

                render_device.poll(wgpu::Maintain::Wait);
                rx.receive().await.unwrap();

                {
                    let u = Mesh::from_raw(&buffer_slice.get_mapped_range());
                    // assets.set(
                    //     transfer.destination,
                    //     U::from_raw(&buffer_slice.get_mapped_range()),
                    // );
                }

                buffer.unmap();
            }
            .block_on();
        }

        Ok(())
    }
}
