use std::marker::PhantomData;

use bevy::{
    prelude::World,
    render::{
        render_graph,
        render_resource::{CommandEncoderDescriptor, MapMode},
        renderer::{RenderContext, RenderQueue},
    },
};

use crate::{
    gpu_insert::{GpuInsertCommand, GpuInsertSender},
    GpuInsert,
};

/// `RenderGraph` node staging data-fed `staging_buffers` making them readable by the Cpu.
pub struct StagingNode<T>(PhantomData<fn() -> T>);

impl<T> Default for StagingNode<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T> render_graph::Node for StagingNode<T>
where
    T: GpuInsert,
    T: 'static,
{
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let gpu_insert_commands = world.resource::<Vec<GpuInsertCommand<T>>>();
        let transfer_sender = world.resource::<GpuInsertSender<T>>();

        // IMPORTANT! create command queue to submit early. See below.
        let mut encoder = render_context
            .render_device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        for command in gpu_insert_commands.iter() {
            encoder.copy_buffer_to_buffer(
                &command.buffer,
                command.bounds.start,
                &command.staging_buffer,
                command.staging_buffer_offset,
                command.bounds.end - command.bounds.start,
            );
        }

        // IMPORTANT! Submit commands to the GPU before staging buffer is staged by submitting `map_async` commands on the main command queue.
        let render_queue = world.resource::<RenderQueue>();
        render_queue.submit(std::iter::once(encoder.finish()));

        for command in gpu_insert_commands.iter() {
            let command_clone = command.clone();
            let transfer_sender = transfer_sender.clone();

            let buffer_slice = command.staging_buffer.slice(
                command.staging_buffer_offset
                    ..command.staging_buffer_offset + (command.bounds.end - command.bounds.start),
            );

            buffer_slice.map_async(MapMode::Read, move |result| {
                result.unwrap();
                transfer_sender.try_send(command_clone).unwrap();
            });
        }

        Ok(())
    }
}
