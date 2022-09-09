use bevy::{
    prelude::{App, CoreStage, Plugin},
    render::{RenderApp, RenderStage},
};
pub use compute::graph::StagingNode;
use gpu_insert::{clear_gpu_insert_commands, insert};
pub use gpu_insert::{GpuInsert, GpuInsertCommand, GpuInsertError, InsertNextFrame};
use std::marker::PhantomData;

pub mod compute;
pub mod gpu_insert;

/// [`Insert`](GpuInsert::insert) data to the `MainWorld` from buffers on the Gpu by issuing [`GpuInsertCommands<T>`](GpuInsertCommand) where `T` implements [`GpuInsert`].
/// Data to be read will be copied to `staging_buffers` to be staged - making them readable by the Cpu.
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
