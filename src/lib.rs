use std::marker::PhantomData;

use bevy::{
    asset::Asset,
    prelude::{AddAsset, App, CoreStage, Mesh, Plugin},
    render::{
        self,
        render_asset::{RenderAsset, RenderAssetPlugin},
        render_graph::RenderGraph,
        render_resource::PrimitiveTopology,
        RenderApp, RenderStage,
    },
};
pub use compute::graph::TransferNode;

pub mod compute;
mod from_raw;
mod mirror_handle;
pub mod transfer;

pub use from_raw::FromRaw;
use transfer::{
    extract_transfers, prepare_transfers, queue_extract_transfers, resolve_pending_transfers,
    PrepareNextFrameTransfers,
};
pub use transfer::{GpuTransfer, Transfer, TransferDescriptor, Transferable};

pub struct TransferPlugin<T, U>
where
    T: RenderAsset,
    T::PreparedAsset: Transferable,
    U: Asset + FromRaw,
{
    marker: PhantomData<fn(T) -> U>,
}

impl<T, U> Default for TransferPlugin<T, U>
where
    T: RenderAsset,
    T::PreparedAsset: Transferable,
    U: Asset + FromRaw,
{
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<T, U> Plugin for TransferPlugin<T, U>
where
    T: RenderAsset,
    T::PreparedAsset: Transferable,
    U: Asset + FromRaw,
{
    fn build(&self, app: &mut App) {
        app.init_resource::<Vec<Transfer<T, U>>>()
            // RenderApp is sub app to the App and is run after the App Schedule (App Stages)
            // could also be in First after marking?
            .add_system_to_stage(CoreStage::First, resolve_pending_transfers::<T, U>)
            .add_system_to_stage(CoreStage::Last, queue_extract_transfers::<T, U>);

        let (sender, receiver) = transfer::create_transfer_channels::<T, U>();
        app.insert_resource(receiver);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(sender)
                .init_resource::<PrepareNextFrameTransfers<T, U>>()
                .init_resource::<Vec<GpuTransfer<T, U>>>()
                .add_system_to_stage(RenderStage::Extract, extract_transfers::<T, U>)
                .add_system_to_stage(RenderStage::Prepare, prepare_transfers::<T, U>);
        }
    }
}
