use bevy::asset::HandleId;
use bevy::render::render_asset::RenderAssetPlugin;
use bevy::render::render_graph::RenderGraph;
use bevy::render::{RenderApp, RenderStage};
use bevy::{prelude::*, render};
use bevy_gpu_insert::{GpuInsertPlugin, TransferNode};
use compute::graph::GenerateMeshNode;
use compute::pipeline::GenerateMeshPipeline;
use generate_mesh::{
    clear_generate_mesh_commands, clear_gpu_generate_mesh_commands, extract_generate_mesh_commands,
    prepare_generate_mesh_commands, queue_generate_mesh_command_bind_groups,
};
use into_render_asset::IntoRenderAssetPlugin;

mod compute;
mod generate_mesh;
mod generated_mesh;
pub mod into_render_asset;

pub use generate_mesh::{
    GenerateMeshCommand, GenerateMeshCommandBindGroups, GpuGenerateMeshCommand,
};
use generated_mesh::{extract_generated_mesh, GeneratedMesh};

struct GenerateMeshPlugin;

impl Plugin for GenerateMeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<GeneratedMesh>()
            .add_plugin(IntoRenderAssetPlugin::<GeneratedMesh>::default())
            .add_plugin(GpuInsertPlugin::<GeneratedMesh>::default())
            .add_system_to_stage(CoreStage::First, clear_generate_mesh_commands);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<GenerateMeshPipeline>()
                .init_resource::<GenerateMeshCommandBindGroups>()
                .add_system_to_stage(RenderStage::Extract, extract_generate_mesh_commands)
                .add_system_to_stage(RenderStage::Extract, extract_generated_mesh)
                .add_system_to_stage(RenderStage::Prepare, prepare_generate_mesh_commands)
                .add_system_to_stage(RenderStage::Queue, queue_generate_mesh_command_bind_groups)
                .add_system_to_stage(RenderStage::Cleanup, clear_gpu_generate_mesh_commands);

            let generate_terrain_mesh_node = GenerateMeshNode::new();
            let transfer_node = TransferNode::<GeneratedMesh>::default();

            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();

            render_graph.add_node(
                compute::graph::node::GENERATE_MESH,
                generate_terrain_mesh_node,
            );

            render_graph.add_node("generate_mesh_transfer", transfer_node);

            // is this right?
            render_graph
                .add_node_edge(
                    "generate_mesh_transfer",
                    render::main_graph::node::CAMERA_DRIVER,
                )
                .unwrap();

            render_graph
                .add_node_edge(
                    compute::graph::node::GENERATE_MESH,
                    "generate_mesh_transfer",
                )
                .unwrap();
        }
    }
}

pub struct Vertices;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(GenerateMeshPlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut generate_mesh_commands: ResMut<Vec<GenerateMeshCommand>>,
    generated_meshes: Res<Assets<GeneratedMesh>>,
) {
    let subdivisions = 20;

    let id = HandleId::random::<GeneratedMesh>();
    let mut destination = Handle::weak(id);
    destination.make_strong(&generated_meshes);
    generate_mesh_commands.push(GenerateMeshCommand {
        insert: destination.clone_weak(),
        subdivisions,
    });

    commands.spawn_bundle((
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
