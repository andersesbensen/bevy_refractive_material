//! A simple 3D scene with light shining over a cube sitting on a plane.

use std::f32::consts::PI;

use bevy::core_pipeline::Skybox;
use bevy::{prelude::*, render::view::RenderLayers};
use bevy_refractive_material::{MainCamera, RefractiveMaterial, RefractiveMaterialPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RefractiveMaterialPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_camera)
        .run();
}

fn animate_camera(mut camera_querey: Query<&mut Transform, With<MainCamera>>, time: Res<Time>) {
    if let Ok(mut t) = camera_querey.get_single_mut() {
        t.rotate_around(
            Vec3::ZERO,
            Quat::from_rotation_y(time.delta_seconds() * 0.3),
        );
        t.look_at(Vec3::ZERO, Vec3::Y);
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut water_materials: ResMut<Assets<RefractiveMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // tall cube which goes through the water surface
    let layers = RefractiveMaterial::layers();
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3 {
                x: 1.0,
                y: 4.0,
                z: 1.0,
            }),
            ..default()
        })
        .insert(layers);

    // Small cube above the water
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::CYAN.into()),
            transform: Transform::from_xyz(-2.0, 2.0, 2.0),
            ..default()
        })
        .insert(layers);

    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(shape::Plane::from_size(1000.0).into()),
        material: materials.add(Color::GRAY.into()),
        ..Default::default()
    }).insert(layers);


    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: false,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    }).insert(layers);

    let skybox = Skybox(asset_server.load("textures/Ryfjallet_cubemap_astc4x4.ktx2"));

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-10.0, 3.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        MainCamera,
        skybox,
    ));

    // Make a mirror
    commands.spawn(MaterialMeshBundle {
        transform: Transform::from_xyz(4.0, 2.0, 0.0).with_rotation(Quat::from_rotation_z(PI/2.0)),
        mesh: meshes.add(shape::Plane::from_size(4.0).into()),
        material: water_materials.add(RefractiveMaterial {
            color: Color::rgba(0.9, 1.0, 0.9, 1.0),
            speed: 0.0,
            wavelength: 0.0,
            r0: 0.0,
            ..Default::default()
        }),
        ..Default::default()
    });
}
