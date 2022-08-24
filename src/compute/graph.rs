use std::marker::PhantomData;

use bevy::{
    asset::Asset,
    prelude::World,
    render::{
        render_graph,
        renderer::{RenderContext, RenderQueue},
    },
};
use wgpu::CommandEncoderDescriptor;

use crate::transfer::{PreparedTransfers, TransferSender};

pub mod node {
    pub const TRANSFER: &str = "transfer";
}

pub struct TransferNode<T, U, V>(PhantomData<fn(T, V) -> U>);

impl<T, U, V> Default for TransferNode<T, U, V> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T, U, V> render_graph::Node for TransferNode<T, U, V>
where
    T: Asset,
    U: Asset,
    V: 'static,
{
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let prepared_transfers = world.resource::<PreparedTransfers<T, U, V>>();
        let transfer_sender = world.resource::<TransferSender<T, U, V>>();

        let mut encoder = render_context
            .render_device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        for (_, gpu_transfer) in prepared_transfers.iter() {
            encoder.copy_buffer_to_buffer(
                &gpu_transfer.source,
                gpu_transfer.source_offset,
                &gpu_transfer.destination,
                gpu_transfer.destination_offset,
                gpu_transfer.size,
            );
        }

        let render_queue = world.resource::<RenderQueue>();
        render_queue.submit(std::iter::once(encoder.finish()));

        for (transfer, gpu_transfer) in prepared_transfers.iter() {
            let handle = transfer.destination.clone_weak();
            let buffer = gpu_transfer.destination.clone();
            let transfer_sender = transfer_sender.clone();

            let buffer_slice = gpu_transfer.destination.slice(..);

            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                result.unwrap();
                transfer_sender.try_send((handle, buffer)).unwrap();
            });
        }

        Ok(())
    }
}
