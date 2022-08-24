use std::marker::PhantomData;

use bevy::{
    asset::Asset,
    ecs::system::{StaticSystemParam, SystemParam, SystemParamItem},
    prelude::{Assets, Commands, Deref, DerefMut, Handle, Res, ResMut},
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_resource::Buffer,
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
    marker: PhantomData<fn() -> V>,
}

pub struct GpuTransfer<T: Asset, U: Asset, V> {
    pub source: Buffer,
    pub source_offset: u64,
    pub destination: Buffer,
    pub destination_offset: u64,
    pub size: u64,
    pub marker: PhantomData<fn(T, V) -> U>,
}

pub type PreparedTransfers<T, U, V> = Vec<(Transfer<T, U, V>, GpuTransfer<T, U, V>)>;

impl<T: Asset, U: Asset, V> Clone for Transfer<T, U, V> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone_weak(),
            destination: self.destination.clone_weak(),
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
    ) -> Result<Self, PrepareAssetError<()>>;
}

pub trait IntoTransfer<U, V>
where
    Self: RenderAsset,
    Self: Sized,
    U: Asset,
{
    type Param: SystemParam;

    fn prepare_transfer(
        prepared_asset: &<Self as RenderAsset>::PreparedAsset,
        transfer: &Transfer<Self, U, V>,
        param: &mut SystemParamItem<<Self as IntoTransfer<U, V>>::Param>,
    ) -> Result<GpuTransfer<Self, U, V>, PrepareAssetError<Transfer<Self, U, V>>>;
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
    pub fn new(source: Handle<T>, destination: Handle<U>) -> Self {
        Self {
            source,
            destination,
            marker: PhantomData,
        }
    }
}

pub struct PrepareNextFrameTransfers<T: Asset, U: Asset, V> {
    pub transfers: Vec<Transfer<T, U, V>>,
}

impl<T: Asset, U: Asset, V> Default for PrepareNextFrameTransfers<T, U, V> {
    fn default() -> Self {
        Self {
            transfers: Vec::new(),
        }
    }
}

pub(crate) fn clear_transfers<T, U, V>(mut commands: Commands)
where
    T: Asset,
    U: Asset,
    V: 'static,
{
    commands.insert_resource(Vec::<Transfer<T, U, V>>::new());
}

pub(crate) fn extract_transfers<T, U, V>(
    mut commands: Commands,
    transfers: Extract<Res<Vec<Transfer<T, U, V>>>>,
) where
    T: Asset,
    U: Asset,
{
    commands.insert_resource(transfers.clone());
}

pub(crate) fn prepare_transfers<T, U, V>(
    mut commands: Commands,
    mut transfers: ResMut<Vec<Transfer<T, U, V>>>,
    mut prepare_next_frame_transfers: ResMut<PrepareNextFrameTransfers<T, U, V>>,
    render_assets: Res<RenderAssets<T>>,
    param: StaticSystemParam<<T as IntoTransfer<U, V>>::Param>,
) where
    T: RenderAsset,
    T: IntoTransfer<U, V>,
    U: Asset,
    V: 'static,
{
    let mut param = param.into_inner();
    let mut prepare_next_frame = Vec::new();
    let mut prepared_transfers = Vec::new();

    for transfer in transfers
        .drain(..)
        .chain(prepare_next_frame_transfers.transfers.drain(..))
    {
        match render_assets.get(&transfer.source) {
            Some(render_asset) => {
                let result =
                    { IntoTransfer::prepare_transfer(render_asset, &transfer, &mut param) };

                match result {
                    // Include Transfer within GpuTransfer
                    Ok(gpu_transfer) => {
                        prepared_transfers.push((transfer, gpu_transfer));
                    }
                    Err(PrepareAssetError::RetryNextUpdate(_)) => {
                        prepare_next_frame.push(transfer);
                    }
                }
            }
            _ => prepare_next_frame.push(transfer),
        }
    }

    commands.insert_resource(PrepareNextFrameTransfers {
        transfers: prepare_next_frame,
    });
    commands.insert_resource(prepared_transfers);
}

pub(crate) fn resolve_pending_transfers<T, U, V>(
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
