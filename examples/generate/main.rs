use bevy::asset::HandleId;
use bevy::reflect::TypeUuid;
use bevy::render::render_asset::RenderAssetPlugin;
use bevy::render::render_graph::RenderGraph;
use bevy::render::render_resource::{BufferDescriptor, BufferUsages};
use bevy::render::renderer::RenderDevice;
use bevy::render::{RenderApp, RenderStage};
use bevy::{prelude::*, render};
use bevy_generate_mesh_on_gpu::{FromRaw, Transfer, TransferNode, TransferPlugin};
use compute::graph::GenerateTerrainMeshNode;
use compute::pipeline::GenerateTerrainMeshPipeline;
use compute::{queue_generate_mesh_bind_groups, GenerateTerrainMeshBindGroups};
use wgpu::PrimitiveTopology;

mod compute;
mod generate_mesh;

use generate_mesh::GenerateMesh;

#[derive(TypeUuid)]
#[uuid = "2b6378c3-e473-499f-99b6-7172e6eb0d5a"]
struct GeneratedMesh;

impl FromRaw for GeneratedMesh {
    fn from_raw(data: &[u8]) -> Self {
        let data: Vec<_> = data
            .chunks_exact(4)
            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
            .collect();
        println!("{data:?}");
        GeneratedMesh
    }
}

struct GenerateTerrainMeshPlugin;

impl Plugin for GenerateTerrainMeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<GenerateMesh>()
            .add_asset::<GeneratedMesh>()
            .add_plugin(RenderAssetPlugin::<GenerateMesh>::default())
            .add_plugin(TransferPlugin::<GenerateMesh, GeneratedMesh>::default());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<GenerateTerrainMeshPipeline>()
                .init_resource::<GenerateTerrainMeshBindGroups>()
                .add_system_to_stage(RenderStage::Queue, queue_generate_mesh_bind_groups);

            let generate_terrain_mesh_node = GenerateTerrainMeshNode::new(&mut render_app.world);
            let transfer_node = TransferNode::<GenerateMesh, GeneratedMesh>::default();

            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();

            render_graph.add_node(
                compute::graph::node::GENERATE_TERRAIN_MESH,
                generate_terrain_mesh_node,
            );

            render_graph.add_node("generate_terrain_mesh_transfer", transfer_node);

            render_graph
                .add_node_edge(
                    compute::graph::node::GENERATE_TERRAIN_MESH,
                    render::main_graph::node::CAMERA_DRIVER,
                )
                .unwrap();

            render_graph
                .add_node_edge(
                    "generate_terrain_mesh_transfer",
                    compute::graph::node::GENERATE_TERRAIN_MESH,
                )
                .unwrap();
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(GenerateTerrainMeshPlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut generate_meshes: ResMut<Assets<GenerateMesh>>,
    mut transfers: ResMut<Vec<Transfer<GenerateMesh, GeneratedMesh>>>,
    generated_meshes: Res<Assets<GeneratedMesh>>,
) {
    let staging_buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("staging buffer"),
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        size: 8 * std::mem::size_of::<f32>() as u64 * 4 * 4,
        mapped_at_creation: false,
    });

    let source = generate_meshes.add(GenerateMesh {});
    let id = HandleId::random::<Mesh>();
    let mut destination = Handle::weak(id);
    destination.make_strong(&generated_meshes);

    let transfer = Transfer::new(
        source.clone_weak(),
        destination.clone_weak(),
        staging_buffer,
    );

    transfers.push(transfer);

    commands.spawn_bundle((source, destination));

    const HALF_SIZE: f32 = 10.0;
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..default()
            },
            shadows_enabled: true,
            illuminance: 10000.0,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 10.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            ..default()
        },
        ..default()
    });

    commands.spawn_bundle(PointLightBundle {
        // transform: Transform::from_xyz(5.0, 8.0, 2.0),
        transform: Transform::from_xyz(0.8, 1.0, 0.8),
        point_light: PointLight {
            intensity: 160000000.0,
            color: Color::BLUE,
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });

    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(2.0, 1.0, -2.0)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..default()
    });
}
