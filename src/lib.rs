use bevy::{
    core_pipeline::Skybox,
    log,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{
        camera::RenderTarget,
        mesh::MeshVertexBufferLayout,
        render_resource::{
            AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages,
        },
        view::{Layer, RenderLayers},
    },
    window::WindowResized,
};

// This is the struct that will be passed to your shader
#[derive(Asset, AsBindGroup, TypeUuid, TypePath, Debug, Clone, Default)]
#[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e0"]
pub struct RefractiveMaterial {
    #[uniform(0)]
    pub color: Color,
    #[uniform(0)]
    pub speed: f32,
    #[uniform(0)]
    pub wavelength: f32,
    #[uniform(0)]
    pub __unused1: f32,
    #[uniform(0)]
    pub __unused2: f32,

    #[texture(1)]
    #[sampler(2)]
    pub refraction_texture: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub reflection_texture: Option<Handle<Image>>,
}

impl Material for RefractiveMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/refractive_material.wgsl".into()
    }

    fn specialize(
        _: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _: &MeshVertexBufferLayout,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

pub struct RefractiveMaterialPlugin;

impl RefractiveMaterialPlugin {
    ///
    /// This function returns the render layers which is used for the calculating the reflections
    /// and refractions
    fn render_layer() -> Layer {
        1
    }
}

///
/// Attach this component to the main main camera to
/// allow for calculating the refelections
#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct WaterSurface;

#[derive(Component)]
struct RefractionCam;

#[derive(Component)]
struct ReflectionCam;

fn resize_notificator(
    resize_event: Res<Events<WindowResized>>,
    mut images: ResMut<Assets<Image>>,
    water_materials: Res<Assets<RefractiveMaterial>>,
) {
    let mut reader = resize_event.get_reader();
    for e in reader.iter(&resize_event) {
        let size = Extent3d {
            width: e.width as _,
            height: e.height as _,
            ..default()
        };

        for (_, m) in water_materials.iter() {
            log::info!("Resizing texture {} {}", e.width, e.height);
            if let Some(handle) = &m.refraction_texture {
                images.get_mut(handle).unwrap().resize(size)
            }
            if let Some(handle) = &m.reflection_texture {
                images.get_mut(handle).unwrap().resize(size)
            }
        }
    }
}

fn build_water(
    mut commands: Commands,
    mut materials: ResMut<Assets<RefractiveMaterial>>,
    mut images: ResMut<Assets<Image>>,
    material: Query<&Handle<RefractiveMaterial>, Changed<Handle<RefractiveMaterial>>>,
    window_query: Query<&Window>,
    main_camera: Query<&Skybox, With<MainCamera>>,
) {
    if let Ok(handle) = material.get_single() {
        if let Some(water) = materials.get_mut(handle) {
            info!("Building new water surface");
            let window = window_query.get_single().unwrap();

            let size = Extent3d {
                width: window.physical_width(),
                height: window.physical_height(),
                ..default()
            };

            // This is the texture that will be rendered to.
            let mut image = Image {
                texture_descriptor: TextureDescriptor {
                    label: None,
                    size,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Bgra8UnormSrgb,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: TextureUsages::TEXTURE_BINDING
                        | TextureUsages::COPY_DST
                        | TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                },
                ..default()
            };
            image.resize(size);
            let refraction_image_handle = images.add(image.clone());
            let reflection_image_handle = images.add(image);

            // This specifies the layer used for the first pass, which will be attached to the first pass camera and cube.
            let first_pass_layer = RenderLayers::layer(RefractiveMaterialPlugin::render_layer());

            let refraction_cam = commands
                .spawn((
                    Camera3dBundle {
                        camera: Camera {
                            // render before the "main pass" camera
                            order: -1,
                            target: RenderTarget::Image(refraction_image_handle.clone()),
                            user_defined_clipping_plane: Some(Vec4::new(0.0, -1.0, 0.0, 0.0)),
                            ..default()
                        },
                        ..default()
                    },
                    first_pass_layer,
                    RefractionCam,
                ))
                .id();

            let reflection_cam = commands
                .spawn((
                    Camera3dBundle {
                        camera: Camera {
                            // render before the "main pass" camera
                            order: -2,
                            target: RenderTarget::Image(reflection_image_handle.clone()),
                            user_defined_clipping_plane: Some(Vec4::new(0.0, 1.0, 0.0, 0.0)),
                            ..default()
                        },
                        ..default()
                    },
                    first_pass_layer,
                    ReflectionCam,
                ))
                .id();

            water.reflection_texture = Some(reflection_image_handle);
            water.refraction_texture = Some(refraction_image_handle);

            // Add skyboxes if needed.
            if let Ok(skybox) = main_camera.get_single() {
                commands.entity(reflection_cam).insert(skybox.clone());
                commands.entity(refraction_cam).insert(skybox.clone());
            }
        }
    }
}

fn system(
    camera_query: Query<
        &Transform,
        (
            With<MainCamera>,
            Without<ReflectionCam>,
            Without<RefractionCam>,
        ),
    >,
    mut reflectcam_query: Query<
        &mut Transform,
        (
            With<ReflectionCam>,
            Without<RefractionCam>,
            Without<MainCamera>,
        ),
    >,
    mut refraction_query: Query<
        &mut Transform,
        (
            With<RefractionCam>,
            Without<ReflectionCam>,
            Without<MainCamera>,
        ),
    >,
) {
    if let Ok(t1) = camera_query.get_single() {
        if let Ok(mut ref_trans) = refraction_query.get_single_mut() {
            *ref_trans = t1.clone();
        }
        if let Ok(mut refl_trans) = reflectcam_query.get_single_mut() {
            let mut t2 = t1.clone();
            t2.translation.y = -t2.translation.y;
            t2.rotation.x = -t2.rotation.x;
            t2.rotation.z = -t2.rotation.z;
            *refl_trans = t2;
        }
    }
}

impl Plugin for RefractiveMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<RefractiveMaterial>::default());
        app.add_systems(Update, system);
        app.add_systems(Update, resize_notificator);
        app.add_systems(Update, build_water);
    }
}
