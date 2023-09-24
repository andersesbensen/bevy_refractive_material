//!
//! This crate provides provides a refractive surface material to bevy.
//! 
//! A reflective surface is something that reflects light such as a mirror or a water surface
//! To use this material two things must happen.
//! 
//! Three things must be done to make this work. 
//! 1) The RefractiveMaterialPlugin must be loaded
//! 2) All meshes in the scene must insert the component RefractiveMaterial::layers()
//! 3) Construct a plane with a materail of type RefractiveMaterial
//!
//! Note: this works by placing extra cameras in the scene, this means that the scene
//! is rendered multiple times from diffrent angles. For this reson refractive surfaces 
//! are quite hevay on the GPU. 
//!  
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
    pub r0: f32, //The reflectivity
    #[uniform(0)]
    pub cos_theta_b: f32, //Cosine of the brewster angle

    #[texture(1)]
    #[sampler(2)]
    pub refraction_texture: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub reflection_texture: Option<Handle<Image>>,
    pub plane: Vec4,
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
        //descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

pub struct RefractiveMaterialPlugin;

impl RefractiveMaterial {
    ///
    /// This function returns the render layers which is used for the calculating the reflections
    /// and refractions
    fn render_layer() -> Layer {
        1
    }

    ///
    /// Insert this in all PbrBundle entities
    pub fn layers() -> RenderLayers {
        RenderLayers::from_layers(&[0, Self::render_layer()])
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

/// Mirror a Transform in the plane defined by the 4D Vector(x,y,z,w)
/// a*x + b*y + c*z + w = 0
/// 
fn mirror_transform(transform: &Transform, plane: &Vec4) -> Transform {
    let t = transform.translation;
    let plane_normal = plane.xyz();
    // Find a point on the plane, w is the distance to the plane in the origin
    // the path to the plane is along the plane normal vector.
    let plane_pos = plane_normal.xyz()*plane.w;
    
    // Mirror the translation part of the transform. Here we make a vector from a point on the plane
    // to the tranlation, then this is projected onto the normal vector. Finally we walt 2 times 
    // the projecton vector towards the plane.
    let translation = t - 2.0 * plane_normal.dot(t - plane_pos) * plane_normal;

    // Build the mirrored rotation. 
    // 1) define a quaterinion that has Y plane in coordinate in the direction of the plane normal
    // 2) apply the inverse rotation of see the camera in the plane coordinate system
    // 3) mirror the rotation in tye XZ plane by flipping the y and w components.
    // 4) rotate the mirrored rotation back to the original cordinate system 
    let plane_q = Quat::from_rotation_arc(Vec3::Y, plane_normal);
    let mut q = plane_q.conjugate() * transform.rotation;
    q.y = -q.y;
    q.w = -q.w;
    let rotation = plane_q * q;

    Transform {
        translation,
        rotation,
        ..Default::default()
    }
}

/// Resizes the textures when the window changes
/// 
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

///
/// When a water material is created we spawn the reflection and refraction cameras
fn build_water(
    mut commands: Commands,
    mut materials: ResMut<Assets<RefractiveMaterial>>,
    mut images: ResMut<Assets<Image>>,
    material: Query<(&Handle<RefractiveMaterial>, &Transform), Changed<Handle<RefractiveMaterial>>>,
    window_query: Query<&Window>,
    main_camera: Query<&Skybox, With<MainCamera>>,
) {
    if let Ok((handle, transform)) = material.get_single() {
        if let Some(water) = materials.get_mut(handle) {
            debug!("Building new water surface");
            let plane_normal = transform.rotation * Vec3::Y;
            let w = transform.translation.dot(plane_normal);
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
            let first_pass_layer = RenderLayers::layer(RefractiveMaterial::render_layer());
            let refraction_cam = commands
                .spawn((
                    Camera3dBundle {
                        camera: Camera {
                            // render before the "main pass" camera
                            order: -1,
                            target: RenderTarget::Image(refraction_image_handle.clone()),
                            user_defined_clipping_plane: Some(Vec4::new(
                                -plane_normal.x,
                                -plane_normal.y,
                                -plane_normal.z,
                                w,
                            )),
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
                            user_defined_clipping_plane: Some(Vec4::new(
                                plane_normal.x,
                                plane_normal.y,
                                plane_normal.z,
                                w,
                            )),
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

            // Small cube above the water

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
            Changed<Transform>,
            Without<ReflectionCam>,
            Without<RefractionCam>,
        ),
    >,
    mut reflectcam_query: Query<
        (&mut Transform, &Camera),
        (
            With<ReflectionCam>,
            Without<RefractionCam>,
            Without<MainCamera>,
        ),
    >,
    mut refraction_query: Query<
        (&mut Transform, &Camera),
        (
            With<RefractionCam>,
            Without<ReflectionCam>,
            Without<MainCamera>,
        ),
    >,
) {
    if let Ok(t1) = camera_query.get_single() {
        if let Ok((mut ref_trans, camera)) = refraction_query.get_single_mut() {
            *ref_trans = t1.clone();
        }
        if let Ok((mut refl_trans, camera)) = reflectcam_query.get_single_mut() {
            *refl_trans = mirror_transform(t1,&camera.user_defined_clipping_plane.unwrap());
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
