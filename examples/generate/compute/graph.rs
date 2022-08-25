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
use crate::GenerateMeshCommandBindGroups;

pub mod node {
    pub const GENERATE_MESH: &str = "generate_mesh";
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
        let GenerateMeshCommandBindGroups { bind_groups } =
            world.resource::<GenerateMeshCommandBindGroups>();

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<GenerateMeshPipeline>();

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

                    for (subdivisions, bind_group) in bind_groups.iter() {
                        pass.set_bind_group(0, bind_group, &[]);
                        println!("dispatch");
                        pass.dispatch_workgroups(*subdivisions, *subdivisions, 1);
                    }
                }
            }
        }

        let render_queue = world.resource::<RenderQueue>();
        render_queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}
