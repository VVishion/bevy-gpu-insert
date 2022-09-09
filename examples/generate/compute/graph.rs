use bevy::{
    prelude::World,
    render::{
        render_graph,
        render_resource::{CachedPipelineState, ComputePassDescriptor, PipelineCache},
        renderer::{RenderContext, RenderQueue},
    },
};
use wgpu::CommandEncoderDescriptor;

use super::pipeline::GenerateMeshPipeline;
use crate::generate_mesh::GenerateMeshDispatch;

pub mod node {
    pub const GENERATE_MESH: &str = "generate_mesh";
    pub const STAGE_GENERATED_MESH: &str = "stage_generated_mesh";
}

enum ComputePipelineState {
    Loading,
    Ready,
}

pub(crate) struct GenerateMeshNode {
    state: ComputePipelineState,
}

impl GenerateMeshNode {
    pub fn new() -> Self {
        Self {
            state: ComputePipelineState::Loading,
        }
    }
}

impl render_graph::Node for GenerateMeshNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<GenerateMeshPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

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
        let dispatches = world.resource::<Vec<GenerateMeshDispatch>>();

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<GenerateMeshPipeline>();

        // IMPORTANT! create command queue to submit early. See below.
        let mut encoder = render_context
            .render_device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());

            // If pipeline is not ready commands are missed.
            match self.state {
                ComputePipelineState::Loading => {}
                ComputePipelineState::Ready => {
                    let pipeline = pipeline_cache
                        .get_compute_pipeline(pipeline.pipeline)
                        .unwrap();
                    pass.set_pipeline(pipeline);

                    for dispatch in dispatches.iter() {
                        pass.set_bind_group(0, &dispatch.bind_group, &[]);
                        pass.dispatch_workgroups(
                            dispatch.workgroups.x,
                            dispatch.workgroups.y,
                            dispatch.workgroups.z,
                        );
                    }
                }
            }
        }

        // IMPORTANT! Submit commands to the GPU before staging buffer is staged by submitting `map_async` commands on the main command queue.
        let render_queue = world.resource::<RenderQueue>();
        render_queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}
