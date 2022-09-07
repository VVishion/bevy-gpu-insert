use std::marker::PhantomData;

use bevy::{
    prelude::{App, CoreStage, Plugin},
    render::{RenderApp, RenderStage},
};
pub use compute::graph::TransferNode;

pub mod compute;
pub mod gpu_insert;

use gpu_insert::{clear_gpu_insert_commands, insert};
pub use gpu_insert::{GpuInsert, GpuInsertCommand, GpuInsertError, InsertNextFrame};

pub struct GpuInsertPlugin<T>
where
    T: GpuInsert,
{
    marker: PhantomData<fn() -> T>,
}

impl<T> Default for GpuInsertPlugin<T>
where
    T: GpuInsert,
{
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<T> Plugin for GpuInsertPlugin<T>
where
    T: GpuInsert,
    T: 'static,
{
    fn build(&self, app: &mut App) {
        app.init_resource::<InsertNextFrame<T>>()
            // RenderApp is sub app to the App and is run after the App Schedule (App Stages)
            .add_system_to_stage(CoreStage::First, insert::<T>);

        let (sender, receiver) = gpu_insert::create_transfer_channels::<T>();
        app.insert_resource(receiver);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(sender)
                .init_resource::<Vec<GpuInsertCommand<T>>>()
                .add_system_to_stage(RenderStage::Cleanup, clear_gpu_insert_commands::<T>);
        }
    }
}
