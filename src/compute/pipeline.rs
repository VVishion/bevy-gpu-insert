use std::borrow::Cow;

use bevy::{
    prelude::{AssetServer, FromWorld, World},
    render::{
        render_resource::{
            BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
            BufferBindingType, CachedComputePipelineId, ComputePipelineDescriptor, PipelineCache,
            SamplerBindingType, ShaderStages, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
    },
};

pub struct GenerateTerrainMeshPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub pipeline: CachedComputePipelineId,
}

impl FromWorld for GenerateTerrainMeshPipeline {
    fn from_world(world: &mut World) -> Self {
        let bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        // BindGroupLayoutEntry {
                        //     binding: 0,
                        //     visibility: ShaderStages::COMPUTE,
                        //     ty: BindingType::Buffer {
                        //         ty: BufferBindingType::Storage { read_only: true },
                        //         has_dynamic_offset: false,
                        //         min_binding_size: None,
                        //     },
                        //     count: None,
                        // },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });
        let shader = world.resource::<AssetServer>().load("generate.wgsl");
        let mut pipeline_cache = world.resource_mut::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: Some(vec![bind_group_layout.clone()]),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("main"),
        });

        GenerateTerrainMeshPipeline {
            bind_group_layout,
            pipeline,
        }
    }
}
