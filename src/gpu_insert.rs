use std::ops::Range;

use bevy::{
    ecs::system::{StaticSystemParam, SystemParam, SystemParamItem},
    prelude::{Commands, Res, ResMut},
    render::render_resource::{Buffer, BufferAddress},
};
use crossbeam_channel::{Receiver, Sender};
use std::ops::Deref;

/// Sender in the `RenderWorld` for [`GpuInsertCommands`](GpuInsertCommand) after the `staging_buffer` was staged (readable).
pub struct GpuInsertSender<T>(pub Sender<GpuInsertCommand<T>>)
where
    T: GpuInsert;

/// Receiver in the `MainWorld` of [`GpuInsertCommands`](GpuInsertCommand) after the `staging_buffer` was staged (readable).
pub struct GpuInsertReceiver<T>(pub Receiver<GpuInsertCommand<T>>)
where
    T: GpuInsert;

impl<T> Clone for GpuInsertSender<T>
where
    T: GpuInsert,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Deref for GpuInsertSender<T>
where
    T: GpuInsert,
{
    type Target = Sender<GpuInsertCommand<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Deref for GpuInsertReceiver<T>
where
    T: GpuInsert,
{
    type Target = Receiver<GpuInsertCommand<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) fn create_transfer_channels<T>() -> (GpuInsertSender<T>, GpuInsertReceiver<T>)
where
    T: GpuInsert,
{
    let (s, r) = crossbeam_channel::unbounded();
    (GpuInsertSender(s), GpuInsertReceiver(r))
}

/// Issue an [`insert`](GpuInsert::insert) with data from `buffer` copied to `staging_buffer`  to be staged (readable) for the `MainWorld`.
///
/// Data from `buffer` within the `bounds` will be copied to the `staging_buffer` starting at the `staging_buffer_offset`.
///
/// Dispatched by pushing a [`GpuInsertCommand`] for `T` implementing [`GpuInsert`] to the resource [`Vec<GpuInsertCommand<T>>`] in the `RenderWorld`.
pub struct GpuInsertCommand<T>
where
    T: GpuInsert,
{
    pub buffer: Buffer,
    pub bounds: Range<BufferAddress>,
    pub staging_buffer: Buffer,
    pub staging_buffer_offset: BufferAddress,
    pub info: T::Info,
}

impl<T> Clone for GpuInsertCommand<T>
where
    T: GpuInsert,
{
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            bounds: self.bounds.clone(),
            staging_buffer: self.staging_buffer.clone(),
            staging_buffer_offset: self.staging_buffer_offset,
            info: self.info.clone(),
        }
    }
}

pub enum GpuInsertError {
    RetryNextUpdate,
}

/// `Insert` data to the `MainWorld` from staged (readable) buffers on the Gpu.
pub trait GpuInsert {
    /// Data required to complete the `insert`.
    /// It will be passed forth from the [`GpuInsertCommand`] issuing this `insert` to [`GpuInsert::insert`].
    type Info: Clone + Send + Sync;
    /// Access ECS data required to complete the `insert` within [`GpuInsert::insert`].
    /// Use [`lifetimeless`](bevy::ecs::system::lifetimeless) [`SystemParam`] for convenience.
    type Param: SystemParam;

    /// Insert data into the `MainWorld`.
    fn insert(
        data: &[u8],
        info: Self::Info,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<(), GpuInsertError>;
}

/// Failed `inserts` to be scheduled for the next frame.
pub struct InsertNextFrame<T>
where
    T: GpuInsert,
{
    pub commands: Vec<GpuInsertCommand<T>>,
}

impl<T> Default for InsertNextFrame<T>
where
    T: GpuInsert,
{
    fn default() -> Self {
        Self {
            commands: Default::default(),
        }
    }
}

/// Clear completed [`GpuInsertCommands`](GpuInsertCommand).
pub(crate) fn clear_gpu_insert_commands<T>(mut commands: Commands)
where
    T: GpuInsert,
    T: 'static,
{
    commands.insert_resource(Vec::<GpuInsertCommand<T>>::new());
}

/// Tries to conclude [`GpuInsertCommands`](GpuInsertCommand) for `T` by [`inserting`](GpuInsert::insert) data from staged (readable) buffers to the `MainWorld`.
///
/// Failed `inserts` will be scheduled for the next frame.
pub(crate) fn insert<T>(
    transfer_receiver: Res<GpuInsertReceiver<T>>,
    mut insert_next_frame: ResMut<InsertNextFrame<T>>,
    param: StaticSystemParam<T::Param>,
) where
    T: GpuInsert,
    T: 'static,
{
    let mut param = param.into_inner();
    let mut queued_transfers = std::mem::take(&mut insert_next_frame.commands);

    for command in queued_transfers
        .drain(..)
        .chain(transfer_receiver.try_iter())
    {
        let buffer_slice = command.staging_buffer.slice(
            command.staging_buffer_offset
                ..command.staging_buffer_offset + (command.bounds.end - command.bounds.start),
        );

        let result = {
            T::insert(
                &buffer_slice.get_mapped_range(),
                command.info.clone(),
                &mut param,
            )
        };

        match result {
            Ok(_) => {
                command.staging_buffer.unmap();
            }
            Err(GpuInsertError::RetryNextUpdate) => {
                insert_next_frame.commands.push(command);
            }
        }
    }
}
