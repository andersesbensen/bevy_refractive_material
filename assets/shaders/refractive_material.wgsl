#import bevy_pbr::mesh_vertex_output MeshVertexOutput
#import bevy_pbr::mesh_view_bindings view;
#import bevy_pbr::mesh_view_bindings globals;

#import bevy_pbr::pbr_types as pbr_types
#import bevy_pbr::pbr_functions as pbr_functions

struct ReflectiveMaterial {
    color: vec4<f32>,
    speed: f32,
    wavelength: f32,

    r0 :f32, //The reflectivity
    cos_theta_b : f32  //Cosine of the brewster angle  
};

@group(1) @binding(0)
var<uniform> material: ReflectiveMaterial;
@group(1) @binding(1)
var refraction_texture: texture_2d<f32>;
@group(1) @binding(2)
var refraction_sampler: sampler;

@group(1) @binding(3)
var reflection_texture: texture_2d<f32>;
@group(1) @binding(4)
var reflection_sampler: sampler;

const PI : f32 = 3.1415926535897932;

//speed
const  speed : f32 = 0.2;
const  speed_x : f32 = 0.3;
const  speed_y : f32 = 0.3;

// refraction
const  emboss :f32 =0.50;
const  intensity :f32= 2.4;
const  steps :u32 = 8u;
const  frequency :f32 =6.0;
const  angle :u32 = 7u; // better when a prime

// reflection
const  delta = 60.;
const  gain = 700.;
const  reflectionCutOff = 0.012;
const  reflectionIntensity = 200000.;

fn col(coord: vec2<f32>, time: f32) -> f32{
    let delta_theta = 2.0 * PI / f32(angle);
    var col = 0.0;
    var theta = 0.0;

    for (var i = 0u; i < steps; i = i + 1u) {
        var adjc = coord;
        theta = delta_theta * f32(i);
        adjc.x += cos(theta) * time * speed + time * speed_x;
        adjc.y -= sin(theta) * time * speed - time * speed_y;
        col = col + cos((adjc.x * cos(theta) - adjc.y * sin(theta)) * frequency) * intensity;
    }
    return cos(col);
}


fn get_uv(pos : vec2<f32>, res : vec2<f32> ) -> vec3<f32>
{
    let time:f32 = globals.time*1.3;

    let p = vec2(
        pos.x / res.x,  // viewport(x_origin, y_origin, width, height)
        pos.y / res.y
    );

    var c1 = p;
    var c2 = p;
    let cc1 = col(c1,time);

    c2.x += res.x/delta;
    let  dx = emboss*(cc1-col(c2,time))/delta;

    c2.x = p.x;
    c2.y += res.y/delta;

    let dy = emboss*(cc1-col(c2,time))/delta;

    c1.x += dx*2.;
    c1.y = -(c1.y+dy*2.);

    var alpha = 1.+ dx*dy * gain;
	
    let ddx = dx - reflectionCutOff;
    let ddy = dy - reflectionCutOff;
    if (ddx > 0. && ddy > 0.) {
        alpha = pow(alpha,ddx*ddy*reflectionIntensity);
    }
    return vec3(c1,alpha);
}


 
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
    let t = material.speed * globals.time;
    let wave = vec2(sin(uv.x * w + t), cos(uv.y * w + t));
    let uv_d = uv + wave * 0.002;
    
    let uv_m = vec2( uv_d.x, 1.0-uv_d.y);

    let refraction = textureSample(refraction_texture, refraction_sampler, uv_d);
    let reflection = textureSample(reflection_texture, reflection_sampler, uv_m);
    
    /// Calculate the direction vector from the camera to 
    /// to the fragment
    let d = view.world_position - mesh.world_position.xyz;
    
    /// As for now the normal vector of the water surface is purly in y
    let transparency = pow(dot(mesh.world_normal, normalize(d)), 0.1)*material.r0;

    /*let light_position = vec3(4.0, 8.0, 4.0);
    let light_dir = normalize(light_position - mesh.world_position.xyz);
    let normal = mesh.world_normal + vec3(wave.y, 0.0, wave.x) * 0.2;
    let light = clamp(dot(normal, light_dir) + 0.5, 0.0, 1.0);*/

    let color = mix(reflection, refraction * material.color, transparency); // * vec4(light);
    return color;
}