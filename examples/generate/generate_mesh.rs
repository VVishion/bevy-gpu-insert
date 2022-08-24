use std::marker::PhantomData;

use bevy::{
    core::cast_slice,
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    reflect::TypeUuid,
    render::{
        render_asset::{PrepareAssetError, RenderAsset},
        render_resource::{Buffer, BufferDescriptor, BufferInitDescriptor, BufferUsages},
        renderer::RenderDevice,
    },
};

use bevy_transfer::{GpuTransfer, IntoTransfer, Transfer};

use crate::{generated_mesh::GeneratedMesh, Vertices};

#[derive(Clone, TypeUuid)]
#[uuid = "cd1cb232-71b1-4b63-878e-6730732911d1"]
pub struct GenerateMesh {
    pub subdivisions: u32,
}

pub struct GpuGenerateMesh {
    pub buffer: Buffer,
    pub subdivisions_buffer: Buffer,
    pub subdivisions: u32,
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
        let subdivisions = extracted_asset.subdivisions;

        let size = 8
            * std::mem::size_of::<f32>() as u64
            * (subdivisions + 1) as u64
            * (subdivisions + 1) as u64;

        let buffer = render_device.create_buffer(&BufferDescriptor {
            usage: BufferUsages::VERTEX
                | BufferUsages::STORAGE
                | BufferUsages::COPY_DST
                | BufferUsages::COPY_SRC,
            label: Some("generate mesh buffer"),
            size,
            mapped_at_creation: false,
        });

        let subdivisions_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::STORAGE,
            label: Some("generate mesh divisions buffer"),
            contents: cast_slice(&[subdivisions]),
        });

        Ok(GpuGenerateMesh {
            buffer,
            subdivisions_buffer,
            subdivisions,
            size,
        })
    }
}

impl IntoTransfer<GeneratedMesh, Vertices> for GenerateMesh {
    type Param = SRes<RenderDevice>;

    fn prepare_transfer(
        prepared_asset: &GpuGenerateMesh,
        _: &Transfer<Self, GeneratedMesh, Vertices>,
        render_device: &mut SystemParamItem<<Self as IntoTransfer<GeneratedMesh, Vertices>>::Param>,
    ) -> Result<
        GpuTransfer<Self, GeneratedMesh, Vertices>,
        PrepareAssetError<Transfer<Self, GeneratedMesh, Vertices>>,
    > {
        let staging_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("staging buffer"),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            size: prepared_asset.size,
            mapped_at_creation: false,
        });

        Ok(GpuTransfer::<Self, GeneratedMesh, Vertices> {
            source: prepared_asset.buffer.clone(),
            source_offset: 0,
            destination: staging_buffer,
            destination_offset: 0,
            size: prepared_asset.size,
            marker: PhantomData,
        })
    }
}
