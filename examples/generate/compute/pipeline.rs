use std::borrow::Cow;

use bevy::{
    prelude::{FromWorld, World},
    render::{
        render_resource::{
            BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
            BufferBindingType, CachedComputePipelineId, ComputePipelineDescriptor, PipelineCache,
            ShaderStages,
        },
        renderer::RenderDevice,
    },
};

use crate::GENERATE_MESH_COMPUTE_SHADER_HANDLE;

pub struct GenerateMeshPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub pipeline: CachedComputePipelineId,
}

impl FromWorld for GenerateMeshPipeline {
    fn from_world(world: &mut World) -> Self {
        let bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let mut pipeline_cache = world.resource_mut::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: Some(vec![bind_group_layout.clone()]),
            shader: GENERATE_MESH_COMPUTE_SHADER_HANDLE.typed().into(),
            shader_defs: vec![],
            entry_point: Cow::from("main"),
        });

        GenerateMeshPipeline {
            bind_group_layout,
            pipeline,
        }
    }
}
