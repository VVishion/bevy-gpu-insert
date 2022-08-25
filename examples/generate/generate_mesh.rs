use bevy::{
    core::cast_slice,
    prelude::{Commands, Handle, Res, ResMut},
    reflect::TypeUuid,
    render::{
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, Buffer, BufferDescriptor,
            BufferInitDescriptor, BufferUsages,
        },
        renderer::RenderDevice,
        Extract,
    },
};

use bevy_gpu_insert::GpuInsertCommand;

use crate::{compute::pipeline::GenerateMeshPipeline, generated_mesh::GeneratedMesh};

#[derive(Clone, TypeUuid)]
#[uuid = "cd1cb232-71b1-4b63-878e-6730732911d1"]
pub struct GenerateMeshCommand {
    pub insert: Handle<GeneratedMesh>,
    pub subdivisions: u32,
}

#[derive(Clone)]
pub struct GpuGenerateMeshCommand {
    pub buffer: Buffer,
    pub subdivisions_buffer: Buffer,
    pub subdivisions: u32,
    pub size: u64,
}

pub(crate) fn clear_generate_mesh_commands(mut commands: Commands) {
    commands.insert_resource(Vec::<GenerateMeshCommand>::new());
}

pub(crate) fn clear_gpu_generate_mesh_commands(mut commands: Commands) {
    commands.insert_resource(Vec::<GpuGenerateMeshCommand>::new());
}

pub(crate) fn extract_generate_mesh_commands(
    mut commands: Commands,
    generate_mesh_commands: Extract<Res<Vec<GenerateMeshCommand>>>,
) {
    commands.insert_resource(generate_mesh_commands.clone());
}

// must be called before prepare transfer
pub(crate) fn prepare_generate_mesh_commands(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut generate_mesh_commands: ResMut<Vec<GenerateMeshCommand>>,
    mut gpu_insert_commands: ResMut<Vec<GpuInsertCommand<GeneratedMesh>>>,
) {
    let mut gpu_generate_mesh_commands = Vec::new();

    for command in generate_mesh_commands.drain(..) {
        let subdivisions = command.subdivisions;

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

        let staging_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("staging buffer"),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            size,
            mapped_at_creation: false,
        });

        gpu_insert_commands.push(GpuInsertCommand {
            insert: command.insert.clone_weak(),
            buffer: buffer.clone(),
            bounds: 0..size,
            staging_buffer,
            staging_buffer_offset: 0,
        });

        gpu_generate_mesh_commands.push(GpuGenerateMeshCommand {
            buffer,
            subdivisions_buffer,
            subdivisions,
            size,
        });
    }

    commands.insert_resource(gpu_generate_mesh_commands);
}

#[derive(Default)]
pub struct GenerateMeshCommandBindGroups {
    pub bind_groups: Vec<(u32, BindGroup)>,
}

pub(crate) fn queue_generate_mesh_command_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<GenerateMeshPipeline>,
    gpu_generate_mesh_commands: Res<Vec<GpuGenerateMeshCommand>>,
) {
    let mut bind_groups = Vec::new();

    for gpu_command in gpu_generate_mesh_commands.iter() {
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: gpu_command.subdivisions_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: gpu_command.buffer.as_entire_binding(),
                },
            ],
            label: Some("generate mesh command bind group"),
            layout: &pipeline.bind_group_layout,
        });

        bind_groups.push((gpu_command.subdivisions, bind_group));
    }

    commands.insert_resource(GenerateMeshCommandBindGroups { bind_groups });
}
