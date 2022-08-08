use bevy::app::{App, Plugin};
use bevy::asset::{Asset, AssetEvent, Assets, Handle, HandleId};
use bevy::ecs::{
    prelude::*,
    system::{StaticSystemParam, SystemParam, SystemParamItem},
};
use bevy::reflect::TypeUuid;
use bevy::render::render_asset::{PrepareAssetError, PrepareAssetLabel, RenderAsset};
use bevy::render::{Extract, RenderApp, RenderStage};
use bevy::utils::{HashMap, HashSet};
use std::marker::PhantomData;

/// Describes how an asset gets extracted and prepared for rendering into a [`RenderAsset::PreparedAsset`] of an existing RenderAsset.
///
/// In the [`RenderStage::Extract`](crate::RenderStage::Extract) step the asset is transferred
/// from the "app world" into the "render world".
/// Therefore it is converted into a [`RenderAsset::ExtractedAsset`], which may be the same type
/// as the render asset itself.
///
/// After that in the [`RenderStage::Prepare`](crate::RenderStage::Prepare) step the extracted asset
/// is transformed into its GPU-representation of type [`RenderAsset::PreparedAsset`].
pub trait IntoRenderAsset: Asset {
    /// The representation of the asset in the "render world".
    type ExtractedAsset: Send + Sync + 'static;
    /// The GPU-representation of the asset.
    type Into: RenderAsset;
    /// Specifies all ECS data required by [`RenderAsset::prepare_asset`].
    /// For convenience use the [`lifetimeless`](bevy_ecs::system::lifetimeless) [`SystemParam`].
    type Param: SystemParam;
    /// Converts the asset into a [`RenderAsset::ExtractedAsset`].
    fn extract_asset(&self) -> Self::ExtractedAsset;
    /// Prepares the `extracted asset` for the GPU by transforming it into
    /// a [`RenderAsset::PreparedAsset`]. Therefore ECS data may be accessed via the `param`.
    fn prepare_asset_into(
        extracted_asset: Self::ExtractedAsset,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<<Self::Into as RenderAsset>::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>>;
}

/// This plugin extracts the changed assets from the "app world" into the "render world"
/// and prepares them for the GPU. They can then be accessed from the [`RenderAssets`] resource.
///
/// Therefore it sets up the [`RenderStage::Extract`](crate::RenderStage::Extract) and
/// [`RenderStage::Prepare`](crate::RenderStage::Prepare) steps for the specified [`RenderAsset`].
pub struct IntoRenderAssetPlugin<A: IntoRenderAsset> {
    prepare_asset_label: PrepareAssetLabel,
    phantom: PhantomData<fn() -> A>,
}

impl<A: IntoRenderAsset> IntoRenderAssetPlugin<A> {
    pub fn with_prepare_asset_label(prepare_asset_label: PrepareAssetLabel) -> Self {
        Self {
            prepare_asset_label,
            phantom: PhantomData,
        }
    }
}

impl<A: IntoRenderAsset> Default for IntoRenderAssetPlugin<A> {
    fn default() -> Self {
        Self {
            prepare_asset_label: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<A: IntoRenderAsset> Plugin for IntoRenderAssetPlugin<A> {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            let prepare_asset_system = prepare_assets::<A>.label(self.prepare_asset_label.clone());

            let prepare_asset_system = match self.prepare_asset_label {
                PrepareAssetLabel::PreAssetPrepare => prepare_asset_system,
                PrepareAssetLabel::AssetPrepare => {
                    prepare_asset_system.after(PrepareAssetLabel::PreAssetPrepare)
                }
                PrepareAssetLabel::PostAssetPrepare => {
                    prepare_asset_system.after(PrepareAssetLabel::AssetPrepare)
                }
            };

            render_app
                .init_resource::<ExtractedAssets<A>>()
                .init_resource::<IntoRenderAssets<A>>()
                .init_resource::<PrepareNextFrameAssets<A>>()
                .add_system_to_stage(RenderStage::Extract, extract_asset_handles::<A>)
                .add_system_to_stage(RenderStage::Extract, extract_render_asset::<A>)
                .add_system_to_stage(RenderStage::Prepare, prepare_asset_system);
        }
    }
}

/// Temporarily stores the extracted and removed assets of the current frame.
pub struct ExtractedAssets<A: IntoRenderAsset> {
    extracted: Vec<(Handle<A>, A::ExtractedAsset)>,
    removed: Vec<Handle<A>>,
}

impl<A: IntoRenderAsset> Default for ExtractedAssets<A> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
        }
    }
}

/// Stores all GPU representations ([`RenderAsset::PreparedAssets`](RenderAsset::PreparedAsset))
/// of [`RenderAssets`](RenderAsset) as long as they exist.
pub type IntoRenderAssets<A> = HashMap<
    Handle<<A as IntoRenderAsset>::Into>,
    <<A as IntoRenderAsset>::Into as RenderAsset>::PreparedAsset,
>;

/// This system extracts all crated or modified assets of the corresponding [`RenderAsset`] type
/// into the "render world".
fn extract_render_asset<A: IntoRenderAsset>(
    mut commands: Commands,
    mut events: Extract<EventReader<AssetEvent<A>>>,
    assets: Extract<Res<Assets<A>>>,
) {
    let mut changed_assets = HashSet::default();
    let mut removed = Vec::new();
    for event in events.iter() {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                changed_assets.insert(handle.clone_weak());
            }
            AssetEvent::Removed { handle } => {
                changed_assets.remove(handle);
                removed.push(handle.clone_weak());
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for handle in changed_assets.drain() {
        if let Some(asset) = assets.get(&handle) {
            extracted_assets.push((handle, asset.extract_asset()));
        }
    }

    commands.insert_resource(ExtractedAssets {
        extracted: extracted_assets,
        removed,
    });
}

// TODO: consider storing inside system?
/// All assets that should be prepared next frame.
pub struct PrepareNextFrameAssets<A: IntoRenderAsset> {
    assets: Vec<(Handle<A>, A::ExtractedAsset)>,
}

impl<A: IntoRenderAsset> Default for PrepareNextFrameAssets<A> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}

/// This system prepares all assets of the corresponding [`RenderAsset`] type
/// which where extracted this frame for the GPU.
fn prepare_assets<R: IntoRenderAsset>(
    mut extracted_assets: ResMut<ExtractedAssets<R>>,
    mut render_assets: ResMut<IntoRenderAssets<R>>,
    mut prepare_next_frame: ResMut<PrepareNextFrameAssets<R>>,
    param: StaticSystemParam<<R as IntoRenderAsset>::Param>,
) {
    let mut param = param.into_inner();
    let mut queued_assets = std::mem::take(&mut prepare_next_frame.assets);
    for (handle, extracted_asset) in queued_assets.drain(..) {
        match R::prepare_asset_into(extracted_asset, &mut param) {
            Ok(prepared_asset) => {
                let id = match handle.id {
                    HandleId::AssetPathId(_) => {
                        panic!("Shouldn't be preparing pending (not-loaded) assets.");
                    }
                    HandleId::Id(_, id) => id,
                };

                let handle_id = HandleId::new(<R as IntoRenderAsset>::Into::TYPE_UUID, id);
                let into_handle = Handle::weak(handle_id.into());
                render_assets.insert(into_handle, prepared_asset);
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                prepare_next_frame.assets.push((handle, extracted_asset));
            }
        }
    }

    for removed in std::mem::take(&mut extracted_assets.removed) {
        let id = match removed.id {
            HandleId::AssetPathId(_) => {
                panic!("Shouldn't be removing pending (not-loaded) assets.");
            }
            HandleId::Id(_, id) => id,
        };

        let handle_id = HandleId::new(<R as IntoRenderAsset>::Into::TYPE_UUID, id);
        let into_handle = Handle::weak(handle_id.into());
        render_assets.remove(&into_handle);
    }

    for (handle, extracted_asset) in std::mem::take(&mut extracted_assets.extracted) {
        match R::prepare_asset_into(extracted_asset, &mut param) {
            Ok(prepared_asset) => {
                let id = match handle.id {
                    HandleId::AssetPathId(_) => {
                        panic!("Shouldn't be preparing pending (not-loaded) assets.");
                    }
                    HandleId::Id(_, id) => id,
                };

                let handle_id = HandleId::new(<R as IntoRenderAsset>::Into::TYPE_UUID, id);
                let into_handle = Handle::weak(handle_id.into());
                render_assets.insert(into_handle, prepared_asset);
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                prepare_next_frame.assets.push((handle, extracted_asset));
            }
        }
    }
}

pub fn extract_asset_handles<R: IntoRenderAsset>(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(Entity, &Handle<R>)>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, handle) in query.iter() {
        let id = match handle.id {
            HandleId::AssetPathId(_) => {
                continue;
            }
            HandleId::Id(_, id) => id,
        };

        let handle_id = HandleId::new(<R as IntoRenderAsset>::Into::TYPE_UUID, id.clone());
        let into_handle = Handle::<R::Into>::weak(handle_id.into());
        values.push((entity, (into_handle, handle.clone())));
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}
