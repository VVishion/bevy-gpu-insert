use std::ops::Range;

use bevy::{
    asset::Asset,
    ecs::system::{StaticSystemParam, SystemParam, SystemParamItem},
    prelude::{Assets, Commands, Handle, Res, ResMut},
    render::{
        render_asset::PrepareAssetError,
        render_resource::{Buffer, BufferAddress},
    },
};
use crossbeam_channel::{Receiver, Sender};
use std::ops::Deref;

pub struct GpuInsertSender<T>(pub Sender<GpuInsertCommand<T>>)
where
    T: GpuInsert;

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

pub fn create_transfer_channels<T>() -> (GpuInsertSender<T>, GpuInsertReceiver<T>)
where
    T: GpuInsert,
{
    let (s, r) = crossbeam_channel::unbounded();
    (GpuInsertSender(s), GpuInsertReceiver(r))
}

pub struct GpuInsertCommand<T>
where
    T: GpuInsert,
{
    pub insert: Handle<T>,
    pub buffer: Buffer,
    pub bounds: Range<BufferAddress>,
    pub staging_buffer: Buffer,
    pub staging_buffer_offset: BufferAddress,
    // pub info: T::Info
}

impl<T> Clone for GpuInsertCommand<T>
where
    T: GpuInsert,
{
    fn clone(&self) -> Self {
        Self {
            insert: self.insert.clone_weak(),
            buffer: self.buffer.clone(),
            bounds: self.bounds.clone(),
            staging_buffer: self.staging_buffer.clone(),
            staging_buffer_offset: self.staging_buffer_offset,
        }
    }
}

pub trait GpuInsert
where
    Self: Asset,
    Self: Sized,
{
    // type Info: Clone;
    type Param: SystemParam;

    fn insert(
        data: &[u8],
        // info: Info,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<()>>;
}

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

pub(crate) fn clear_gpu_insert_commands<T>(mut commands: Commands)
where
    T: GpuInsert,
{
    commands.insert_resource(Vec::<GpuInsertCommand<T>>::new());
}

pub(crate) fn insert<T>(
    transfer_receiver: Res<GpuInsertReceiver<T>>,
    mut insert_next_frame: ResMut<InsertNextFrame<T>>,
    mut assets: ResMut<Assets<T>>,
    param: StaticSystemParam<T::Param>,
) where
    T: GpuInsert,
{
    let mut param = param.into_inner();
    let mut queued_transfers = std::mem::take(&mut insert_next_frame.commands);

    let mut resolve = |command: GpuInsertCommand<T>| {
        //                                                          0..gpu_transfer.size
        let buffer_slice = command.staging_buffer.slice(..);

        let result = { T::insert(&buffer_slice.get_mapped_range(), &mut param) };

        match result {
            Ok(asset) => {
                let _ = assets.set(command.insert, asset);
                command.staging_buffer.unmap();
            }
            Err(PrepareAssetError::RetryNextUpdate(_)) => {
                insert_next_frame.commands.push(command);
            }
        }
    };

    for command in queued_transfers
        .drain(..)
        .chain(transfer_receiver.try_iter())
    {
        resolve(command);
    }
}
