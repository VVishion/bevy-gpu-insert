use bevy::asset::HandleId;
use bevy::prelude::*;
use bevy::render::render_resource::{BufferDescriptor, BufferUsages};
use bevy::render::renderer::RenderDevice;
use bevy_generate_mesh_on_gpu::{FromRaw, GenerateMesh, GenerateMeshPlugin, Transfer};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(GenerateMeshPlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut generate_meshes: ResMut<Assets<GenerateMesh>>,
    mut transfers: ResMut<Vec<Transfer<GenerateMesh, Mesh>>>,
    meshes: Res<Assets<Mesh>>,
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
    destination.make_strong(&meshes);

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
