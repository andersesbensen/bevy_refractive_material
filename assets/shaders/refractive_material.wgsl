#import bevy_pbr::mesh_vertex_output MeshVertexOutput
#import bevy_pbr::mesh_view_bindings view;
#import bevy_pbr::mesh_view_bindings globals;

#import bevy_pbr::pbr_types as pbr_types
#import bevy_pbr::pbr_functions as pbr_functions

struct WaterMaterial {
    color: vec4<f32>,
    speed: f32,
    wavelength : f32,
    unused1 : f32,
    unused2 : f32,
};

@group(1) @binding(0)
var<uniform> material: WaterMaterial;
@group(1) @binding(1)
var refraction_texture: texture_2d<f32>;
@group(1) @binding(2)
var refraction_sampler: sampler;

@group(1) @binding(3)
var reflection_texture: texture_2d<f32>;
@group(1) @binding(4)
var reflection_sampler: sampler;

 
 @fragment
fn fragment(
    mesh: MeshVertexOutput
) -> @location(0) vec4<f32> {
    let uv = vec2(
        mesh.position.x / view.viewport[2],  // viewport(x_origin, y_origin, width, height)
        mesh.position.y / view.viewport[3]
    );
    
    // Make some displacement of the texture sampling such that it looks like waves
    //
    let w = material.wavelength;
    let t = material.speed*globals.time;
    let uv_d = uv + vec2( sin(uv.x*w + t),cos(uv.y*w + t) )*0.002;
    
    let uv_m = vec2(uv_d.x,1.0 -uv_d.y);

    let refraction = textureSample(refraction_texture, refraction_sampler, uv_d );
    let reflection = textureSample(reflection_texture, reflection_sampler, uv_m );
    
    /// Calculate the direction vector from the camera to 
    /// to the fragment
    let d = view.world_position-mesh.world_position.xyz;
    
    /// As for now the normal vector of the water surface is purly in y
    let transparency = pow(dot(mesh.world_normal, normalize(d)),0.1);
    
    let color = mix(reflection,refraction*material.color,transparency);

    return color;
}