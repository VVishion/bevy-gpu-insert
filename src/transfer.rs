use std::marker::PhantomData;

use bevy::{
    asset::Asset,
    prelude::{Assets, Commands, Handle, Res, ResMut},
    render::{
        render_asset::{RenderAsset, RenderAssets},
        render_resource::{Buffer, BufferAddress, BufferId},
        Extract,
    },
    utils::HashSet,
};
use crossbeam_channel::{Receiver, Sender};
use std::ops::Deref;

use crate::FromRaw;

pub struct TransferSender<T, U: Asset>(pub Sender<(Handle<U>, Buffer)>, PhantomData<fn(T) -> U>);

pub struct TransferReceiver<T, U: Asset>(
    pub Receiver<(Handle<U>, Buffer)>,
    PhantomData<fn(T) -> U>,
);

impl<T, U: Asset> Clone for TransferSender<T, U> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<T, U: Asset> Deref for TransferSender<T, U> {
    type Target = Sender<(Handle<U>, Buffer)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, U: Asset> Deref for TransferReceiver<T, U> {
    type Target = Receiver<(Handle<U>, Buffer)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn create_transfer_channels<T, U: Asset>() -> (TransferSender<T, U>, TransferReceiver<T, U>) {
    let (s, r) = crossbeam_channel::unbounded();
    (
        TransferSender(s, PhantomData),
        TransferReceiver(r, PhantomData),
    )
}

// Could include staging buffer layout

pub struct Transfer<T: Asset, U: Asset> {
    pub source: Handle<T>,
    pub destination: Handle<U>,
    pub staging_buffer: Buffer,
}

impl<T: Asset, U: Asset> Clone for Transfer<T, U> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone_weak(),
            destination: self.destination.clone_weak(),
            staging_buffer: self.staging_buffer.clone(),
        }
    }
}

pub struct BufferCopy<T: Asset, U: Asset> {
    pub source: Buffer,
    pub source_offset: u64,
    pub destination: Buffer,
    pub destination_offset: u64,
    pub size: u64,
    marker: PhantomData<fn(T) -> U>,
}

pub struct BufferMap<T: Asset, U: Asset> {
    pub buffer: Buffer,
    marker: PhantomData<fn(T) -> U>,
}

pub trait Transferable {
    fn get_transfer_descriptors(&self) -> Vec<TransferDescriptor>;
}

pub struct TransferDescriptor {
    // maybe put slice here
    pub buffer: Buffer,
    pub size: BufferAddress,
}

impl<T: Asset, U: Asset> Transfer<T, U> {
    pub fn new(source: Handle<T>, destination: Handle<U>, staging_buffer: Buffer) -> Self {
        Self {
            source,
            destination,
            staging_buffer,
        }
    }
}
pub struct ExtractTransfers<T: Asset, U: Asset> {
    pub extract: Vec<Transfer<T, U>>,
}

impl<T: Asset, U: Asset> Default for ExtractTransfers<T, U> {
    fn default() -> Self {
        Self {
            extract: Vec::new(),
        }
    }
}

pub struct PrepareNextFrameTransfers<T: Asset, U: Asset> {
    pub transfers: Vec<Transfer<T, U>>,
}

impl<T: Asset, U: Asset> Default for PrepareNextFrameTransfers<T, U> {
    fn default() -> Self {
        Self {
            transfers: Vec::new(),
        }
    }
}

pub type BufferCopies<T, U> = Vec<BufferCopy<T, U>>;

pub struct BufferMaps<T: Asset, U: Asset> {
    pub maps: Vec<(Handle<U>, BufferMap<T, U>)>,
}

#[derive(Default)]
pub struct BufferUnmaps<T, U> {
    pub buffers: Vec<BufferId>,
    marker: PhantomData<fn(T) -> U>,
}

#[derive(Default)]
pub struct MappedBuffers {
    pub buffers: HashSet<BufferId>,
}

impl<T: Asset, U: Asset> Default for BufferMaps<T, U> {
    fn default() -> Self {
        Self { maps: Vec::new() }
    }
}

pub(crate) fn queue_extract_transfers<T, U>(
    mut commands: Commands,
    mut transfers: ResMut<Vec<Transfer<T, U>>>,
) where
    T: Asset,
    U: Asset,
{
    commands.insert_resource(ExtractTransfers {
        extract: transfers.drain(..).collect(),
    })
}

pub(crate) fn extract_transfers<T, U>(
    mut commands: Commands,
    transfers: Extract<Res<ExtractTransfers<T, U>>>,
) where
    T: Asset,
    U: Asset,
{
    commands.insert_resource(transfers.extract.clone());
}

pub(crate) fn prepare_transfers<T, U>(
    mut commands: Commands,
    mut mapped_buffers: ResMut<MappedBuffers>,
    mut transfers: ResMut<Vec<Transfer<T, U>>>,
    mut prepare_next_frame_transfers: ResMut<PrepareNextFrameTransfers<T, U>>,
    render_assets: Res<RenderAssets<T>>,
) where
    T: RenderAsset,
    T::PreparedAsset: Transferable,
    U: Asset,
{
    let mut prepare_next_frame = Vec::new();
    let mut copies = Vec::new();
    let mut maps = Vec::new();

    for transfer in transfers
        .drain(..)
        .chain(prepare_next_frame_transfers.transfers.drain(..))
    {
        let buffer_id = transfer.staging_buffer.id();

        match render_assets.get(&transfer.source) {
            Some(render_asset) if !mapped_buffers.buffers.contains(&buffer_id) => {
                mapped_buffers.buffers.insert(transfer.staging_buffer.id());

                let mut offset = 0;

                for transfer_descriptor in render_asset.get_transfer_descriptors() {
                    copies.push(BufferCopy::<T, U> {
                        source: transfer_descriptor.buffer,
                        source_offset: 0,
                        destination: transfer.staging_buffer.clone(),
                        destination_offset: offset,
                        size: transfer_descriptor.size,
                        marker: PhantomData,
                    });

                    offset += transfer_descriptor.size;
                }

                maps.push((
                    transfer.destination.clone_weak(),
                    BufferMap::<T, U> {
                        buffer: transfer.staging_buffer.clone(),
                        marker: PhantomData,
                    },
                ));
            }
            _ => prepare_next_frame.push(transfer),
        }
    }

    commands.insert_resource(PrepareNextFrameTransfers {
        transfers: prepare_next_frame,
    });
    commands.insert_resource(copies);
    commands.insert_resource(BufferMaps { maps });
}

pub(crate) fn resolve_pending_transfers<T, U>(
    mut commands: Commands,
    transfer_receiver: Res<TransferReceiver<T, U>>,
    mut assets: ResMut<Assets<U>>,
) where
    T: Asset,
    U: Asset + FromRaw,
{
    let mut unmapped_buffers = Vec::new();

    if let Ok((handle, buffer)) = transfer_receiver.try_recv() {
        let buffer_slice = buffer.slice(..);

        {
            let _ = assets.set(handle, U::from_raw(&buffer_slice.get_mapped_range()));
        }

        buffer.unmap();
        unmapped_buffers.push(buffer.id());
    }

    commands.insert_resource(BufferUnmaps::<T, U> {
        buffers: unmapped_buffers,
        marker: PhantomData,
    });
}

pub(crate) fn extract_unmaps<T, U>(
    unmapped_buffers: Extract<Res<BufferUnmaps<T, U>>>,
    mut mapped_buffers: ResMut<MappedBuffers>,
) {
    for buffer_id in unmapped_buffers.buffers.iter() {
        mapped_buffers.buffers.remove(buffer_id);
    }
}
