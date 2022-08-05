use bevy::{
    asset::{Asset, HandleId},
    prelude::Handle,
};

pub trait MirrorHandle {
    fn mirror<T: Asset>(&self) -> Handle<T>;
}

impl<T: Asset> MirrorHandle for Handle<T> {
    #[inline]
    fn mirror<U: Asset>(&self) -> Handle<U> {
        if self.is_strong() {
            panic!("Is it safe to mirror strong handles? I doubt it..");
        }

        let id = match self.id {
            HandleId::AssetPathId(_) => {
                panic!("Can't mirror handles of pending assets.");
            }
            HandleId::Id(_, id) => id,
        };

        let mirrored = HandleId::new(U::TYPE_UUID, id);
        Handle::<U>::weak(mirrored.into())
    }
}
