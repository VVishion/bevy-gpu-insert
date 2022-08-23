use std::marker::PhantomData;

use bevy::{
    asset::Asset,
    ecs::system::{StaticSystemParam, SystemParam, SystemParamItem},
    prelude::{Assets, Commands, Deref, DerefMut, Handle, Res, ResMut},
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_resource::{Buffer, BufferAddress, BufferDescriptor, BufferUsages},
        renderer::RenderDevice,
        Extract,
    },
};
use crossbeam_channel::{Receiver, Sender};
use std::ops::Deref;

pub struct TransferSender<T, U: Asset, V>(
    pub Sender<(Handle<U>, Buffer)>,
    PhantomData<fn(T, V) -> U>,
);

pub struct TransferReceiver<T, U: Asset, V>(
    pub Receiver<(Handle<U>, Buffer)>,
    PhantomData<fn(T, V) -> U>,
);

impl<T, U: Asset, V> Clone for TransferSender<T, U, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<T, U: Asset, V> Deref for TransferSender<T, U, V> {
    type Target = Sender<(Handle<U>, Buffer)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, U: Asset, V> Deref for TransferReceiver<T, U, V> {
    type Target = Receiver<(Handle<U>, Buffer)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn create_transfer_channels<T, U: Asset, V>(
) -> (TransferSender<T, U, V>, TransferReceiver<T, U, V>) {
    let (s, r) = crossbeam_channel::unbounded();
    (
        TransferSender(s, PhantomData),
        TransferReceiver(r, PhantomData),
    )
}

pub struct Transfer<T: Asset, U: Asset, V> {
    pub source: Handle<T>,
    pub destination: Handle<U>,
    pub size: u64,
    marker: PhantomData<fn() -> V>,
}

pub struct GpuTransfer<T: Asset, U: Asset, V> {
    pub source: Buffer,
    pub source_offset: u64,
    pub destination: Buffer,
    pub destination_offset: u64,
    pub size: u64,
    marker: PhantomData<fn(T, V) -> U>,
}

pub type PreparedTransfers<T, U, V> = Vec<(Handle<U>, GpuTransfer<T, U, V>)>;

impl<T: Asset, U: Asset, V> Clone for Transfer<T, U, V> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone_weak(),
            destination: self.destination.clone_weak(),
            size: self.size,
            marker: PhantomData,
        }
    }
}

pub trait FromTransfer<T, V>
where
    T: Asset,
    Self: Asset,
    Self: Sized,
{
    type Param: SystemParam;

    fn from(
        data: &[u8],
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self>>;
}

pub struct TransferDescriptor {
    pub buffer: Buffer,
    pub size: BufferAddress,
}

pub trait IntoTransfer<U, V> {
    fn into(&self) -> TransferDescriptor;
}

pub struct ResolveNextFrameTransfers<T, U, V>
where
    U: Asset,
{
    pub transfers: Vec<(Handle<U>, Buffer)>,
    marker: PhantomData<fn(T, V) -> U>,
}

impl<T, U, V> Default for ResolveNextFrameTransfers<T, U, V>
where
    U: Asset,
{
    fn default() -> Self {
        Self {
            transfers: Default::default(),
            marker: PhantomData,
        }
    }
}

impl<T: Asset, U: Asset, V> Transfer<T, U, V> {
    pub fn new(source: Handle<T>, destination: Handle<U>, size: u64) -> Self {
        Self {
            source,
            destination,
            size,
            marker: PhantomData,
        }
    }
}

#[derive(Deref, DerefMut, Clone)]
pub struct Queue<T>(pub Vec<T>);

pub struct PrepareNextFrameTransfers<T: Asset, U: Asset, V> {
    pub transfers: Queue<Transfer<T, U, V>>,
}

impl<T: Asset, U: Asset, V> Default for PrepareNextFrameTransfers<T, U, V> {
    fn default() -> Self {
        Self {
            transfers: Queue(Vec::new()),
        }
    }
}

pub(crate) fn queue_extract_transfers<T, U, V>(
    mut commands: Commands,
    mut transfers: ResMut<Vec<Transfer<T, U, V>>>,
) where
    T: Asset,
    U: Asset,
    V: 'static,
{
    commands.insert_resource(Queue(transfers.drain(..).collect()));
}

pub(crate) fn extract_transfers<T, U, V>(
    mut commands: Commands,
    transfers: Extract<Res<Queue<Transfer<T, U, V>>>>,
) where
    T: Asset,
    U: Asset,
{
    commands.insert_resource(transfers.clone());
}

pub(crate) fn prepare_transfers<T, U, V>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut transfers: ResMut<Queue<Transfer<T, U, V>>>,
    mut prepare_next_frame_transfers: ResMut<PrepareNextFrameTransfers<T, U, V>>,
    render_assets: Res<RenderAssets<T>>,
    //param: StaticSystemParam<<R as RenderAsset>::Param>,
) where
    T: RenderAsset,
    T::PreparedAsset: IntoTransfer<U, V>,
    U: Asset,
    V: 'static,
{
    let mut prepare_next_frame = Vec::new();
    let mut prepared_transfers = Vec::new();

    for transfer in transfers
        .drain(..)
        .chain(prepare_next_frame_transfers.transfers.drain(..))
    {
        match render_assets.get(&transfer.source) {
            Some(render_asset) => {
                let staging_buffer = render_device.create_buffer(&BufferDescriptor {
                    label: Some("staging buffer"),
                    usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                    size: transfer.size,
                    mapped_at_creation: false,
                });

                let transfer_descriptor = IntoTransfer::into(render_asset);
                prepared_transfers.push((
                    transfer.destination,
                    GpuTransfer::<T, U, V> {
                        source: transfer_descriptor.buffer,
                        source_offset: 0,
                        destination: staging_buffer,
                        destination_offset: 0,
                        size: transfer_descriptor.size,
                        marker: PhantomData,
                    },
                ));
            }
            _ => prepare_next_frame.push(transfer),
        }
    }

    commands.insert_resource(PrepareNextFrameTransfers {
        transfers: Queue(prepare_next_frame),
    });
    commands.insert_resource(prepared_transfers);
}

pub(crate) fn resolve_pending_transfers<T, U, V>(
    mut commands: Commands,
    transfer_receiver: Res<TransferReceiver<T, U, V>>,
    mut resolve_next_frame: ResMut<ResolveNextFrameTransfers<T, U, V>>,
    mut assets: ResMut<Assets<U>>,
    param: StaticSystemParam<U::Param>,
) where
    T: Asset,
    U: Asset + FromTransfer<T, V>,
    V: 'static,
{
    let mut param = param.into_inner();
    let mut unmapped_buffers = Vec::new();
    let mut queued_transfers = std::mem::take(&mut resolve_next_frame.transfers);

    let mut resolve = |handle, buffer: Buffer| {
        let buffer_slice = buffer.slice(..);

        let result = { U::from(&buffer_slice.get_mapped_range(), &mut param) };

        match result {
            Ok(asset) => {
                let _ = assets.set(handle, asset);
                buffer.unmap();
                unmapped_buffers.push(buffer.id());
            }
            Err(PrepareAssetError::RetryNextUpdate(_)) => {
                resolve_next_frame.transfers.push((handle, buffer));
            }
        }
    };

    for (handle, buffer) in queued_transfers.drain(..) {
        resolve(handle, buffer);
    }

    for (handle, buffer) in transfer_receiver.try_iter() {
        resolve(handle, buffer);
    }
}
