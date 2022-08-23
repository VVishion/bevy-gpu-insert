use std::marker::PhantomData;

use bevy::{
    asset::Asset,
    prelude::{App, CoreStage, Plugin},
    render::{render_asset::RenderAsset, RenderApp, RenderStage},
};
pub use compute::graph::TransferNode;

pub mod compute;
pub mod transfer;

use transfer::{
    extract_transfers, prepare_transfers, queue_extract_transfers, resolve_pending_transfers,
};
pub use transfer::{
    FromTransfer, GpuTransfer, IntoTransfer, PrepareNextFrameTransfers, ResolveNextFrameTransfers,
    Transfer, TransferDescriptor,
};

pub struct TransferPlugin<T, U, V>
where
    T: RenderAsset,
    T::PreparedAsset: IntoTransfer<U, V>,
    U: Asset + FromTransfer<T, V>,
    V: 'static,
{
    marker: PhantomData<fn(T, V) -> U>,
}

impl<T, U, V> Default for TransferPlugin<T, U, V>
where
    T: RenderAsset,
    T::PreparedAsset: IntoTransfer<U, V>,
    U: Asset + FromTransfer<T, V>,
    V: 'static,
{
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<T, U, V> Plugin for TransferPlugin<T, U, V>
where
    T: RenderAsset,
    T::PreparedAsset: IntoTransfer<U, V>,
    U: Asset + FromTransfer<T, V>,
    V: 'static,
{
    fn build(&self, app: &mut App) {
        app.init_resource::<Vec<Transfer<T, U, V>>>()
            .init_resource::<ResolveNextFrameTransfers<T, U, V>>()
            // RenderApp is sub app to the App and is run after the App Schedule (App Stages)
            .add_system_to_stage(CoreStage::First, resolve_pending_transfers::<T, U, V>)
            .add_system_to_stage(CoreStage::Last, queue_extract_transfers::<T, U, V>);

        let (sender, receiver) = transfer::create_transfer_channels::<T, U, V>();
        app.insert_resource(receiver);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(sender)
                .init_resource::<PrepareNextFrameTransfers<T, U, V>>()
                .init_resource::<Vec<GpuTransfer<T, U, V>>>()
                .add_system_to_stage(RenderStage::Extract, extract_transfers::<T, U, V>)
                .add_system_to_stage(RenderStage::Prepare, prepare_transfers::<T, U, V>);
        }
    }
}
