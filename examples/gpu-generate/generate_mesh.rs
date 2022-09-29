use bevy::{
    math::UVec3,
    prelude::{Commands, Handle, Res, ResMut},
    render::{
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, Buffer, BufferDescriptor, BufferUsages,
        },
        renderer::RenderDevice,
        Extract,
    },
};

use bevy_gpu_insert::GpuInsertCommand;

use crate::{compute::pipeline::GenerateMeshPipeline, generated_mesh::GeneratedMesh};

#[derive(Clone)]
pub struct GenerateMeshCommand {
    pub insert: Handle<GeneratedMesh>,
    pub subdivisions: u32,
}

#[derive(Clone)]
pub struct GpuGenerateMeshCommand {
    pub buffer: Buffer,
    pub staging_buffer: Buffer,
    pub subdivisions: u32,
    pub size: u64,
    pub insert: Handle<GeneratedMesh>,
}

pub(crate) fn clear_generate_mesh_commands(mut commands: Commands) {
    commands.insert_resource(Vec::<GenerateMeshCommand>::new());
}

pub(crate) fn extract_generate_mesh_commands(
    mut commands: Commands,
    generate_mesh_commands: Extract<Res<Vec<GenerateMeshCommand>>>,
) {
    commands.insert_resource(generate_mesh_commands.clone());
}

pub(crate) fn prepare_generate_mesh_commands(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut generate_mesh_commands: ResMut<Vec<GenerateMeshCommand>>,
) {
    let mut gpu_generate_mesh_commands = Vec::new();

    for GenerateMeshCommand {
        insert,
        subdivisions,
    } in generate_mesh_commands.drain(..)
    {
        let subdivisions = subdivisions;

        let size = 8
            * std::mem::size_of::<f32>() as u64
            * (subdivisions + 1) as u64
            * (subdivisions + 1) as u64;

        let buffer = render_device.create_buffer(&BufferDescriptor {
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            label: Some("generate mesh buffer"),
            size,
            mapped_at_creation: false,
        });

        let staging_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("staging buffer"),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            size,
            mapped_at_creation: false,
        });

        gpu_generate_mesh_commands.push(GpuGenerateMeshCommand {
            buffer,
            staging_buffer,
            subdivisions,
            size,
            insert,
        });
    }

    commands.insert_resource(gpu_generate_mesh_commands);
}

pub struct GenerateMeshDispatch {
    pub bind_group: BindGroup,
    pub workgroups: UVec3,
}

pub(crate) fn queue_generate_mesh_dispatches(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<GenerateMeshPipeline>,
    gpu_generate_mesh_commands: Res<Vec<GpuGenerateMeshCommand>>,
    mut gpu_insert_commands: ResMut<Vec<GpuInsertCommand<GeneratedMesh>>>,
) {
    let mut dispatches = Vec::new();

    for gpu_command in gpu_generate_mesh_commands.iter() {
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: gpu_command.buffer.as_entire_binding(),
            }],
            label: Some("generate mesh command bind group"),
            layout: &pipeline.bind_group_layout,
        });

        gpu_insert_commands.push(GpuInsertCommand {
            buffer: gpu_command.buffer.clone(),
            bounds: 0..gpu_command.size,
            staging_buffer: gpu_command.staging_buffer.clone(),
            staging_buffer_offset: 0,
            info: gpu_command.insert.clone_weak(),
        });

        dispatches.push(GenerateMeshDispatch {
            bind_group,
            workgroups: UVec3::new(
                gpu_command.subdivisions + 1,
                gpu_command.subdivisions + 1,
                1,
            ),
        });
    }

    commands.insert_resource(dispatches);
}
