use std::marker::PhantomData;

use bevy::{
    asset::Asset,
    prelude::{Mesh, World},
    render::{
        render_graph,
        render_resource::{CachedPipelineState, ComputePassDescriptor, PipelineCache},
        renderer::{RenderContext, RenderQueue},
    },
};
use wgpu::CommandEncoderDescriptor;

use crate::transfer::{PreparedTransfers, TransferSender};

pub mod node {
    pub const TRANSFER: &str = "transfer";
}

pub struct TransferNode<T, U>(PhantomData<fn(T) -> U>);

impl<T, U> Default for TransferNode<T, U> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T, U> render_graph::Node for TransferNode<T, U>
where
    T: Asset,
    U: Asset,
{
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let prepared_transfers = world.resource::<PreparedTransfers<T, U>>();
        let transfer_sender = world.resource::<TransferSender<T, U>>();

        let mut encoder = render_context
            .render_device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        for (_, transfer) in prepared_transfers.prepared.iter() {
            encoder.copy_buffer_to_buffer(
                &transfer.source,
                transfer.source_offset,
                &transfer.destination,
                transfer.destination_offset,
                transfer.size,
            );
        }

        let render_queue = world.resource::<RenderQueue>();
        render_queue.submit(std::iter::once(encoder.finish()));

        for (handle, transfer) in prepared_transfers.prepared.iter() {
            // some transfers may copy to the same staging buffer, so the same staging buffer is mapped multiple times
            let handle = handle.clone_weak();
            let buffer = transfer.destination.clone();
            let transfer_sender = transfer_sender.clone();

            let buffer_slice = transfer.destination.slice(..);

            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                result.unwrap();
                transfer_sender.try_send((handle, buffer)).unwrap();
            });
        }

        Ok(())
    }
}
