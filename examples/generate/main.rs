use bevy::asset::HandleId;
use bevy::render::render_asset::RenderAssetPlugin;
use bevy::render::render_graph::RenderGraph;
use bevy::render::render_resource::{BufferDescriptor, BufferUsages};
use bevy::render::renderer::RenderDevice;
use bevy::render::{RenderApp, RenderStage};
use bevy::{prelude::*, render};
use bevy_generate_mesh_on_gpu::{Transfer, TransferNode, TransferPlugin};
use compute::graph::GenerateMeshNode;
use compute::pipeline::GenerateMeshPipeline;
use compute::{
    extract_generate_mesh_changes, queue_generate_mesh_bind_groups, GenerateMeshBindGroups,
};
use into_render_asset::IntoRenderAssetPlugin;

mod compute;
mod generate_mesh;
mod generated_mesh;
pub mod into_render_asset;

use generate_mesh::GenerateMesh;
use generated_mesh::{extract_generated_mesh, GeneratedMesh};

struct GenerateTerrainMeshPlugin;

impl Plugin for GenerateTerrainMeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<GenerateMesh>()
            .add_asset::<GeneratedMesh>()
            .add_plugin(IntoRenderAssetPlugin::<GeneratedMesh>::default())
            .add_plugin(RenderAssetPlugin::<GenerateMesh>::default())
            .add_plugin(TransferPlugin::<GenerateMesh, GeneratedMesh, VertexData>::default());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<GenerateMeshPipeline>()
                .init_resource::<GenerateMeshBindGroups>()
                .add_system_to_stage(RenderStage::Extract, extract_generate_mesh_changes)
                .add_system_to_stage(RenderStage::Extract, extract_generated_mesh)
                .add_system_to_stage(RenderStage::Queue, queue_generate_mesh_bind_groups);

            let generate_terrain_mesh_node = GenerateMeshNode::new();
            let transfer_node = TransferNode::<GenerateMesh, GeneratedMesh, VertexData>::default();

            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();

            render_graph.add_node(
                compute::graph::node::GENERATE_MESH,
                generate_terrain_mesh_node,
            );

            render_graph.add_node("generate_mesh_transfer", transfer_node);

            render_graph
                .add_node_edge(
                    compute::graph::node::GENERATE_MESH,
                    render::main_graph::node::CAMERA_DRIVER,
                )
                .unwrap();

            render_graph
                .add_node_edge(
                    "generate_mesh_transfer",
                    compute::graph::node::GENERATE_MESH,
                )
                .unwrap();
        }
    }
}

pub struct VertexData;

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
    mut transfers: ResMut<Vec<Transfer<GenerateMesh, GeneratedMesh, VertexData>>>,
    generated_meshes: Res<Assets<GeneratedMesh>>,
) {
    let subdivisions = 20;

    // create the staging buffer in the render world. it will be sent to the main world.
    let staging_buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("staging buffer"),
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        size: 8
            * std::mem::size_of::<f32>() as u64
            * (subdivisions + 1) as u64
            * (subdivisions + 1) as u64,
        mapped_at_creation: false,
    });

    let source = generate_meshes.add(GenerateMesh { subdivisions });
    let id = HandleId::random::<GeneratedMesh>();
    let mut destination = Handle::weak(id);
    destination.make_strong(&generated_meshes);

    let transfer = Transfer::<_, _, VertexData>::new(
        source.clone_weak(),
        destination.clone_weak(),
        staging_buffer,
    );

    transfers.push(transfer);

    commands.spawn_bundle((
        source,
        destination,
        materials.add(StandardMaterial {
            base_color: Color::GREEN,
            //cull_mode: None,
            ..default()
        }),
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
        ComputedVisibility::default(),
    ));

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

    // commands.spawn_bundle(PointLightBundle {
    //     // transform: Transform::from_xyz(5.0, 8.0, 2.0),
    //     transform: Transform::from_xyz(0.8, 1.0, 0.8),
    //     point_light: PointLight {
    //         intensity: 160000000.0,
    //         color: Color::BLUE,
    //         shadows_enabled: true,
    //         ..default()
    //     },
    //     ..default()
    // });

    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-1.0, 5.0, -1.0)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        //projection: Projection::Orthographic(OrthographicProjection::default()),
        ..default()
    });
}
