use bevy::{
    asset::HandleId,
    prelude::*,
    render,
    render::{render_graph::RenderGraph, Extract, RenderApp, RenderStage},
};
use bevy_gpu_insert::{GpuInsertPlugin, StagingNode};
use bevy_into_render_asset::{IntoRenderAsset, IntoRenderAssetPlugin};
use bevy_map_handle::MapHandle;
use compute::{graph::GenerateMeshNode, pipeline::GenerateMeshPipeline};
use generate_mesh::{
    clear_generate_mesh_commands, clear_gpu_generate_mesh_commands, extract_generate_mesh_commands,
    prepare_generate_mesh_commands, queue_generate_mesh_dispatches,
};

mod compute;
mod generate_mesh;
mod generated_mesh;

pub use generate_mesh::{GenerateMeshCommand, GenerateMeshDispatch, GpuGenerateMeshCommand};
use generated_mesh::{extract_generated_mesh, GeneratedMesh};

pub fn extract_generated_mesh_handles(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(Entity, &Handle<GeneratedMesh>)>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, handle) in query.iter() {
        let mapped = match handle.map_weak::<<GeneratedMesh as IntoRenderAsset>::Into>() {
            Err(_) => continue,
            Ok(handle) => handle,
        };

        values.push((entity, (mapped,)));
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}

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
                .add_system_to_stage(RenderStage::Extract, extract_generated_mesh_handles)
                .add_system_to_stage(RenderStage::Extract, extract_generate_mesh_commands)
                .add_system_to_stage(RenderStage::Extract, extract_generated_mesh)
                .add_system_to_stage(RenderStage::Prepare, prepare_generate_mesh_commands)
                .add_system_to_stage(RenderStage::Queue, queue_generate_mesh_dispatches)
                .add_system_to_stage(RenderStage::Cleanup, clear_gpu_generate_mesh_commands);

            let generate_terrain_mesh_node = GenerateMeshNode::new();
            let transfer_node = StagingNode::<GeneratedMesh>::default();

            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();

            render_graph.add_node(
                compute::graph::node::GENERATE_MESH,
                generate_terrain_mesh_node,
            );

            render_graph.add_node(compute::graph::node::STAGE_GENERATED_MESH, transfer_node);

            // is this the right ordering?
            render_graph
                .add_node_edge(
                    compute::graph::node::STAGE_GENERATED_MESH,
                    render::main_graph::node::CAMERA_DRIVER,
                )
                .unwrap();

            render_graph
                .add_node_edge(
                    compute::graph::node::GENERATE_MESH,
                    compute::graph::node::STAGE_GENERATED_MESH,
                )
                .unwrap();
        }
    }
}

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

    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-1.0, 5.0, -1.0)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..default()
    });
}
