use bevy::{
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::{Buffer, BufferDescriptor, BufferUsages},
        renderer::RenderDevice,
    },
};

use bevy_generate_mesh_on_gpu::{TransferDescriptor, Transferable};

#[derive(Clone, TypeUuid)]
#[uuid = "cd1cb232-71b1-4b63-878e-6730732911d1"]
pub struct GenerateMesh {}

pub struct GpuGenerateMesh {
    pub buffer: Buffer,
    pub size: u64,
}

impl RenderAsset for GenerateMesh {
    type ExtractedAsset = GenerateMesh;
    type PreparedAsset = GpuGenerateMesh;
    type Param = SRes<RenderDevice>;

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let size = 8 * std::mem::size_of::<f32>() as u64 * 5 * 5;

        let buffer = render_device.create_buffer(&BufferDescriptor {
            usage: BufferUsages::VERTEX
                | BufferUsages::STORAGE
                | BufferUsages::COPY_DST
                | BufferUsages::COPY_SRC,
            label: Some("generate mesh buffer"),
            size,
            mapped_at_creation: false,
        });

        Ok(GpuGenerateMesh { buffer, size })
    }
}

impl Transferable for GpuGenerateMesh {
    fn get_transfer_descriptors(&self) -> Vec<TransferDescriptor> {
        vec![TransferDescriptor {
            buffer: self.buffer.clone(),
            size: self.size,
        }]
    }
}
