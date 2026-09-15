enable wgpu_binding_array;

struct input {
    @location(0) pos: vec3f,
    @location(1) uv: vec2f,
};

struct output {
    @builtin(position) clip: vec4f,
    @location(0) vUv: vec2f,
}

@vertex
fn main(model: input) -> output {
    var out: output;

    out.vUv = model.uv;
    out.clip = vec4f(model.pos, 1.0);

    return out;
}

////////////////////////////////
// Some code is based on "gpu-tracing" (https://github.com/RayTracing/gpu-tracing)
// Originally written in 2023 by Arman Uguray <arman.uguray@gmail.com>
// licensed under CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/deed.en)

struct Rng {
  state: u32,
};

var<private> rng: Rng;

fn init_rng(pixel: vec2u) {
  let seed = (pixel.x + pixel.y * u32(settings.res.x)) ^ jenkins_hash(u32(settings.frame));
  rng.state = jenkins_hash(seed);
}

fn jenkins_hash(i: u32) -> u32 {
  var x = i;
  x += x << 10u;
  x ^= x >> 6u;
  x += x << 3u;
  x ^= x >> 11u;
  x += x << 15u;
  return x;
}

fn xorshift32() -> u32 {
  var x = rng.state;
  x ^= x << 13;
  x ^= x >> 17;
  x ^= x << 5;
  rng.state = x;
  return x;
}

fn rand_f32() -> f32 {
  return bitcast<f32>(0x3f800000u | (xorshift32() >> 9u)) - 1.;
}

@group(0) @binding(1)
var<uniform> camera: Camera;

struct Camera {
    pos: vec4f,
    forward: vec4f,
    right: vec4f,
    up: vec4f,

    fov: f32,
};

@group(0) @binding(0)
var<uniform> settings: Settings;

struct Settings {
    res: vec2f,
    frame: u32,
}

struct Ray {
    orig: vec3f,
    dir: vec3f,
}

fn at(r: Ray, t: f32) -> vec3f {
    return r.orig + t * r.dir;
}

struct HitRecord {
    normal: vec3f,
    t: f32,
    mat: u32,
    uv: vec2f,
}

struct Scatter {
    atten: vec3f,
    ray: Ray,
}

struct Material {
    color_factor: vec4f,
    color: i32,

    metal_rough: i32,
    metal_factor: f32,
    rough_factor: f32,

    emiss_factor: vec4f,
    emiss: i32,

    ior: f32,
    tran_factor: f32,
    tran: i32,

    atten_color: vec4f,
    atten_dist: f32,
    thick_factor: f32,

    color_sampler: i32,
    metal_rough_sampler: i32,
    emiss_sampler: i32,
    tran_sampler: i32,
}

@group(3) @binding(0)
var textures: binding_array<texture_2d<f32>>;
@group(3) @binding(1)
var samplers: binding_array<sampler>;

fn sample(ind: i32, smp_ind: i32, uv: vec2f) -> vec4f {

    if smp_ind == -1 {
         let size = textureDimensions(textures[ind]);
        let x = u32(fract(uv.x) * f32(size.x - 1));
        let y = u32(fract(uv.y) * f32(size.y - 1));

        return textureLoad(textures[ind], vec2(x, y), 0);
    }

    return textureSample(textures[ind], samplers[smp_ind], uv);
}

struct Sphere {
    center: vec3f,
    rad: f32,
    mat: u32,
}

fn sphere_pdf(orig: vec3f, direction: vec3f, s:Sphere, normal: vec3f) -> f32{
    let hit = sphere_intersect(Ray(orig, direction), s);

    if hit.t <= 0.0 {
        return 0.0;
    }

    var dir = s.center - orig;
    var dsq = dot(dir, dir);

    let ratio = clamp(s.rad * s.rad / dsq, 0.0, 1.0);

    let cost_max = sqrt(1.0 - ratio);
    let solid_angle = 2 * PI  * (1.0 - cost_max);

    return 1 / solid_angle;
}


//Nans if the point is inside the sphere
//temp hack for sphere IS this should be solved asap there is some nasty fp stuff going on here
//I either bump the point out, which is not a solution itself as there could be a case where a point needs to scatter inside the sphere we are IS
//I think its bump (ensure it never ends on the inside) + handle the case where the point is inside the sphere due to edge cases
fn sphere_dirgen(s: Sphere, orig: vec3f, normal: vec3f) -> vec3f {

    var dir = s.center - orig;
    var dsq = dot(dir, dir);

    let ratio = clamp(s.rad * s.rad / dsq, 0.0, 1.0);

    let r1 = rand_f32();
    let r2 = rand_f32();

    let z = 1.0 + r2 * (sqrt(1.0 - ratio) - 1.0);

    let phi = 2 * PI * r1;

    let x = cos(phi) * sqrt(1.0 - z * z);
    let y = sin(phi) * sqrt(1.0 - z * z);

    let local = vec3(x, y, z);

    dir = normalize(dir);
    let tbn = tbn(dir);

    return normalize(tbn * local);
}

struct Triangle {
    a: vec4f,
    b: vec4f,
    c: vec4f,
    norm0: vec4f, //in order a, b, c
    norm1: vec4f,
    norm2: vec4f,
    uv0: vec2f,
    uv1: vec2f,
    uv2: vec2f,
    mat: u32,
}

fn triangle_pdf() {

}

fn triangle_dirgen() {

}

@group(1) @binding(0)
var ping: texture_2d<f32>;
@group(1) @binding(1)
var pong: texture_storage_2d<rgba32float, write>;

struct BvhGPU {
    right: i32,
    typ: u32,
    x: vec2f,
    y: vec2f,
    z: vec2f,
}

@group(2) @binding(0) var<storage> bvh: array<BvhGPU>;
@group(2) @binding(1) var<storage> spheres: array<Sphere>;
@group(2) @binding(2) var<storage> triangles: array<Triangle>;
@group(2) @binding(3) var<storage> materials: array<Material>;

const PI = 3.141;
const EPSILON = 1e-3;
const INF = 3.4e+38;

//Normal Distrubution Function
fn Trowbridge_Reitz_GGX(a2: f32, ndoth: f32) -> f32 {
    let denom = ndoth * ndoth * (a2 - 1.0) + 1.0;
    return check(ndoth) * a2 / (PI * denom * denom);
}

//see derivation in notes
fn sample_ndf(a2: f32, normal: vec3f) -> vec3f {
    let e1 = rand_f32();
    let e2 = rand_f32();

    let cost = sqrt((1.0 - e2) / (e2 * (a2 - 1.0) + 1.0));
    let sint = sqrt(1.0 - min(cost * cost, 1)); //this causes NaNs
    let phi = 2 * PI * e1;

    let norm_tangent = vec3(sint * cos(phi), sint * sin(phi), cost);

    let tbn = tbn(normal);

    return normalize(tbn * norm_tangent);
}

fn sample_vndf(view: vec3f, alpha: f32, normal: vec3f) -> vec3f {

    //first transform view into this space
    let tbn = tbn(normal);
    let vl = view * tbn;

    let vh = normalize(vec3(vl.x * alpha, vl.y * alpha, vl.z));
    let lensq = vh.x * vh.x + vh.y * vh.y;

    let T1 = select(vec3(1, 0, 0), vec3(-vh.y, vh.x, 0) * inverseSqrt(lensq), lensq > 0);
    let T2 = cross(vh, T1);

    let e1 = rand_f32();
    let e2 = rand_f32();

    let r = sqrt(e1);
    let phi = 2 * PI * e2;

    let t1 = r * cos(phi);
    let t2 = r * sin(phi);

    let s = 0.5 * (1 + vh.z);
    let t2i = (1 - s) * sqrt(1 - t1 * t1) + s * t2;

    let nh = t1 * T1 + t2i * T2 + sqrt(max(1 - t1 * t1 - t2i * t2i, 0.0)) * vh;
    let ne = normalize(vec3(nh.x * alpha, nh.y * alpha, max(nh.z, 0.0)));

    return tbn * ne;
}

//isotropic, doesnt matter
fn tbn(normal: vec3f) -> mat3x3f {
    let a = select(vec3(0.0, 0.0, 1.0), vec3(1.0, 0.0, 0.0), abs(normal.z) > 0.9);
    let u = normalize(cross(normal, a));
    let v = normalize(cross(normal, u));

    return mat3x3f(u, v, normal);
}

//vdoth ommitted because of Dv
fn Smith_GGX_G1(a2: f32, ndotx: f32) -> f32 {
    return 2.0 * ndotx / (ndotx + sqrt(a2 + (1.0 - a2) * ndotx * ndotx));
}

fn Height_Correlated_Smith_GGX(a2: f32, ndotv: f32, ndotl: f32, vdoth: f32, ldoth: f32) -> f32 {
    let lv = ndotv * sqrt(a2 + (1.0 - a2) * abs(ndotl) * abs(ndotl));
    let vl = abs(ndotl) * sqrt(a2 + (1.0 - a2) * ndotv * ndotv);

    return check(vdoth) * check(ndotl * ldoth) * (2.0 * abs(ndotl) * ndotv) / (lv + vl);
}

//Fresnel
fn f0(ior: f32) -> f32 {
    return ((ior - 1.) / (ior + 1)) * ((ior - 1.) / (ior + 1));
}

fn fresnel(f0: vec3f, cost: f32, ratio: f32) -> vec3f {
    var cos = cost;

    if ratio > 1.0 {
        let sin2t = ratio * ratio * (1 - cost * cost);
        if sin2t > 1 { return vec3(1); }
        cos = sqrt(max(1 - sin2t, 0.0));
    }

    let u = 1 - cos;
    return mix(f0, vec3(1.0), u * u * u * u * u);
}

//fresnel in the paper
fn dielectric_fresnel(cost: f32, eta: f32) -> f32 {
    let sin2t = eta * eta * (1 - cost * cost);
    if sin2t > 1 { return 1; }
    let costt = sqrt(max(1 - sin2t, 0.0));

    let rs = (eta * costt - cost) / (eta * costt + cost);
    let rp = (eta * cost - costt) / (eta * cost + costt);

    return 0.5 * (rs * rs + rp * rp);
}

fn dead() -> Scatter {
    return Scatter(vec3(0.0), Ray(vec3(0.0), vec3(0.0)));
}

fn dead_record() -> HitRecord {
    return HitRecord(vec3(0.0), 0.0, 0, vec2(0.0));
}

fn check(x: f32) -> f32 {
    return select(0.0, 1.0, x > 0.0);
}

//microfacet BSDF + sampling the next dir
fn bsdf(ray: Ray, hit: HitRecord, mat: Material) -> Scatter {

    let inc = normalize(ray.dir);

    let side = dot(inc, hit.normal) < 0.0;
    let normal = select(-hit.normal, hit.normal, side);

    var color = mat.color_factor.rgb;
    var opacity = mat.color_factor.a;

    if mat.color >= 0 {
        let smp = sample(mat.color, mat.color_sampler, hit.uv);
        color *= smp.rgb;
        opacity *= smp.a;
    }

    var metal = mat.metal_factor;
    var rough = mat.rough_factor;

    if mat.metal_rough >= 0 {
        let smp = sample(mat.metal_rough, mat.metal_rough_sampler, hit.uv);
        metal *= smp.b;
        rough *= smp.g;
    }

    let roughness = clamp(rough, 0.03, 1.0); //if this is zero bad things happen (D becomes 0/0)
    var a = roughness * roughness;
    var a2 = a * a;

    var tran = mat.tran_factor;
    if mat.tran >= 0 {
        tran *= sample(mat.tran, mat.tran_sampler, hit.uv).r;
    }

    var tran_color = color;

    //quicky haks for gltf scenes without tranmission
    if opacity < 1 && tran == 0.0 {
        tran = 1.0;
        tran_color = vec3(1.0);

        a /= 8;
        a2 = a * a;
    }

    let etai = select(mat.ior, 1.0, side);
    let etao = select(1.0, mat.ior, side);

    let ratio = etai / etao;

    let f0 = vec3(f0(mat.ior));
    let thin = mat.thick_factor == 0;

    var scattered: vec3f;
    var atten = vec3(1.0);

    //this is wrong in many many levels
    //doesnt work with nested volumes
    if !side && !thin {
       atten *= pow(mat.atten_color.rgb, vec3(hit.t / (mat.atten_dist * 10.0)));
    }

    //sampling start

    //MIS, using the balance heuristic
    let tran_weight = min(tran, 0.5);

    let rest = 1.0 - tran_weight;

    let spec_weight = rest * 0.5;
    let diff_weight = rest * 0.2;
    let light_weight = rest * 0.3;
    let chance = rand_f32();

    var origin = at(ray, hit.t);


    if chance < tran_weight {
        //TIR? 0.0 atten | tran is 0? 0.0 atten
        let h = sample_vndf(-inc, a, normal);
        //h = sample_ndf(a2, normal);

        //now it matters if the material actually has volume or not given by thick_factor
        //thick_factor > 0 we actually refract this time
        if !thin {
            scattered = refract(inc, h, ratio);
        }

        //thick_factor == 0? there is no refraction buddy its all a lie
        else {
            scattered = reflect(inc, h);
            scattered = reflect(scattered, normal);
        }
    }

    else if chance < spec_weight + tran_weight {
        let h = sample_vndf(-inc, a, normal);
        scattered = normalize(reflect(inc, h));

    } else  if chance < spec_weight + diff_weight + tran_weight {
        scattered = normalize(normal + ssp());
    } else {
        scattered = sphere_dirgen(spheres[1], origin, normal);
    }

    //eval
    let ndotl = dot(normal, scattered);
    let ndotv = dot(normal, -inc);

    let h_r = normalize(-inc + scattered);
    let ndoth_r = dot(normal, h_r);
    let vdoth_r = dot(-inc, h_r);
    let ldoth_r = dot(scattered, h_r);

    var h_t: vec3f;
    var l_t: vec3f;

    if thin {
        l_t = reflect(scattered, normal);
        h_t = normalize(-inc + l_t);
    }

    else {
        l_t = scattered;
        h_t = normalize(etai * -inc + etao * scattered);
        h_t = select(-h_t, h_t, dot(h_t, normal) > 0);
    }

    let ndoth_t = dot(normal, h_t);
    let vdoth_t = dot(-inc, h_t);
    let ldoth_t = dot(l_t, h_t);

    let D_r = Trowbridge_Reitz_GGX(a2, ndoth_r);
    let D_t = Trowbridge_Reitz_GGX(a2, ndoth_t);
    let G1 = Smith_GGX_G1(a2, ndotv);

    let d = etai * vdoth_t + etao * ldoth_t;

    var f: vec3f;

    //brdf
    if ndotl > 0 {

        let dielectric_fresnel = fresnel(f0, vdoth_r, ratio);
        let metal_fresnel = fresnel(color, vdoth_r, ratio);

        let G2 = Height_Correlated_Smith_GGX(a2, ndotv, ndotl, vdoth_r, ldoth_r);

        let specular = D_r * G2 / (4 * ndotl * ndotv);
        let diffuse = color / PI;

        let tran_diffuse =  (1.0 - tran) * diffuse; //this is because the btdf part is separate and not mixed like the spec

        let dielectric_brdf = mix(tran_diffuse, vec3(specular), dielectric_fresnel);
        let metal_brdf = metal_fresnel * specular;

        f = mix(dielectric_brdf, metal_brdf, metal);

    }

    //btdf, with respect to the thin vs volume model in gltf
    else {

        let F = 1 - fresnel(f0, vdoth_t, ratio);

        var frt: f32;
        var G2: f32;

        if thin {
            G2 = Height_Correlated_Smith_GGX(a2, ndotv, -ndotl, vdoth_t, ldoth_t); //l is actually on the other side
            frt =  D_t * G2 / (4 * abs(ndotl) * ndotv);
        }

        else {
            G2 = Height_Correlated_Smith_GGX(a2, ndotv, ndotl, vdoth_t, ldoth_t);
            frt = (abs(vdoth_t) * abs(ldoth_t)) / (ndotv * abs(ndotl)) * etao * etao * D_t * G2 / (d * d); //this also filters bad transmission cuz of the g2 term
        }

        f = (1 - metal) * tran * tran_color * frt * F;
    }

    //pdf
    let spec_pdf = spec_weight * D_r * G1 / (4 * ndotv);// * check(vdoth_r);
    let diff_pdf = diff_weight * (max(ndotl, 0.0) / PI);
    let light_pdf = light_weight * sphere_pdf(origin, scattered, spheres[1], normal);

    var tran_pdf = tran_weight;

    if thin {
        tran_pdf *= D_t * G1 / (4 * ndotv);// * check(vdoth_t);
    }

    else { //this is cursed and im not sure
        tran_pdf *= D_t * G1 * max(vdoth_t, 0.0) / ndotv * etao * etao * max(-ldoth_t, 0.0) / (d * d); //unlike frt above, there is no g2, thus the needed checks
    }

    let pdf = spec_pdf + diff_pdf + light_pdf + tran_pdf;

    atten *= f * abs(ndotl) / pdf;
    let output = Ray(origin, scattered);

    return Scatter(atten, output);
}

fn ssp() -> vec3f {
    let y = 1 - 2 * rand_f32();
    let proj = sqrt(1 - y * y);

    let phi= 6.28318530718 * rand_f32();
    return vec3(proj * cos(phi), y, proj * sin(phi));
}

fn sphere_intersect(r: Ray, s: Sphere) -> HitRecord {
    let oc = s.center - r.orig;
    let a = dot(r.dir, r.dir);
    let h = dot(r.dir, oc);
    let c = dot(oc, oc) - s.rad * s.rad;

    let disc = h * h - a * c;

    if disc < 0 {
        return dead_record();
    }

    let sqrtd = sqrt(disc);
    let root1 = (h - sqrtd) / a;
    let root2 = (h + sqrtd) / a;

    let root = select(root2, root1, root1 > EPSILON);

    if root <= EPSILON { //reject
        return dead_record();
    }

    let normal = normalize((at(r, root) - s.center) / s.rad);

    return HitRecord(normal, root, s.mat, vec2(0.0)); //calc UV
}

fn triangle_intersect(r: Ray, t: Triangle) -> HitRecord {
    let e1 = (t.b - t.a).xyz;
    let e2 = (t.c - t.a).xyz;

    let ray_cross_e2 = cross(r.dir, e2);
    let det = dot(e1, ray_cross_e2);

    if abs(det) < EPSILON {
        return dead_record();
    }

    let inv_det = 1.0 / det;
    let s = r.orig - t.a.xyz;
    let u = inv_det * dot(s, ray_cross_e2);
    if u < 0.0 || u > 1.0 {
        return dead_record();
    }

    let s_cross_e1 = cross(s, e1);
    let v = inv_det * dot(r.dir, s_cross_e1);
    if v < 0.0 || u + v > 1.0 {
        return dead_record();
    }

    let int = inv_det * dot(e2, s_cross_e1);

    if int > EPSILON {

        let normal = (1 - u - v) * t.norm0 + u * t.norm1 + v * t.norm2;
        let uv = (1 - u - v) * t.uv0 + u * t.uv1 + v * t.uv2;

        return HitRecord(normalize(normal.xyz), int, t.mat, uv);
    }

    return dead_record();
}

fn aabb_intersect(r: Ray, interval: vec2f, x: vec2f, y: vec2f, z: vec2f) -> f32 {
    let adinv = 1 / r.dir;

    let mn = vec3(x.r, y.r, z.r);
    let mx = vec3(x.g, y.g, z.g);

    let t0 = (mn - r.orig) * adinv;
    let t1 = (mx - r.orig) * adinv;

    let tmn = min(t0, t1);
    let tmx=  max(t0, t1);

    let first = max(max(tmn.x, interval.x), max(tmn.y, tmn.z));
    let second = min(min(tmx.x, interval.y), min(tmx.y, tmx.z));

    if first <= second {
        return second; //will be posstive bc interval.x
    }

    return -1;
}

//essentially an iterative dfs problem below
fn bvh_intersect(r: Ray) -> HitRecord {

    var dfs: array<i32, 32>; //size limit, a binary serach tree height will be log2n which is not the case with SAH sooo
    var index: i32;
    dfs[0] = 0;
    index = 1;

    var closest = HitRecord(vec3(0.0), INF, 0, vec2(0.0));
    var interval: vec2f;
    interval.x = EPSILON;
    interval.y = INF;

    while index > 0 {
        index -= 1;
        let node = bvh[dfs[index]];

        // let bbox = aabb_intersect(r, interval, node.x, node.y, node.z);

        // if bbox < 0.0 {
        //     continue;
        // }

        if node.typ == 1 { //node
            let left = dfs[index] + 1;
            let right =  node.right;

            let left_node = bvh[left];
            let right_node = bvh[right];

            //use aabb hit distance as another heuristic instead of left then right
            let bbox_left = aabb_intersect(r, interval, left_node.x, left_node.y, left_node.z);
            let bbox_right = aabb_intersect(r, interval, right_node.x, right_node.y, right_node.z);

            var invalid = 0;

            if bbox_left < 0 {
                invalid += 1;
            }

            if bbox_right < 0 {
                invalid += 1;
            }

            //invalid = 0 both  / 1 first = the one that is no < 0 / 2 none
            let first = select(select(right, left, bbox_left <= bbox_right), select(select(left, right, bbox_left < 0), -1, invalid == 2), invalid >= 1);
            let second = select(select(right, left, first == right), -1, invalid >= 1);

            if second >= 0 {
                dfs[index] = second;
                index += 1;
            }

            if first >= 0 {
                dfs[index] = first;
                index += 1;
            }

        } else if node.typ == 3 {
            let t = sphere_intersect(r, spheres[node.right]);
            if t.t > 0 && t.t < closest.t {
                 closest = t;
                 interval.y = min(interval.y, t.t);
             }
        } else if node.typ == 2 {
            let t = triangle_intersect(r, triangles[node.right]);
            if t.t > 0 && t.t < closest.t {
                 closest = t;
                 interval.y = min(interval.y, t.t);
             }
        }
    }

    return closest;
}

@fragment
fn fs_main(in: output) -> @location(0) vec4f {

    init_rng(vec2u(in.clip.xy));

    let height= 2 * tan(camera.fov / 2) * 1;
    let width = height * (settings.res.x / settings.res.y);

    let tweak = vec2(rand_f32() - 0.5, rand_f32() - 0.5) / settings.res;
    var viewport_loc = (2 * (in.vUv + tweak) - 1) * vec2(width, -height);

    let dir = (viewport_loc.x * camera.right + viewport_loc.y * camera.up + 1.0 * camera.forward).xyz; //focus distance is 1
    var ray = Ray(camera.pos.xyz, dir);

    var light = vec3f(1.0);
    var cur = vec3f(0.0);

    let sky = vec3(0.0);

    for(var j= 0; j < 6; j++) {
        let closest = bvh_intersect(ray);

        if closest.t < INF {
        } else {
            cur += sky * light;
            break;
        }

        let scatter_ray = bsdf(ray, closest, materials[closest.mat]);

        var emiss = materials[closest.mat].emiss_factor.rgb;

        if materials[closest.mat].emiss >= 0 {
            emiss *= sample(materials[closest.mat].emiss, materials[closest.mat].emiss_sampler, closest.uv).rgb;
        }

        cur += light * emiss;

        light *= scatter_ray.atten;
        ray = scatter_ray.ray;

        //dot checks debug
        // if scatter_ray.atten.b == 1.0 {
        //     cur = vec3(1.0, 0.0, 0.0);
        //     break;
        // }

        if all(light == vec3(0.0)) {
            break;
        }
    }

    var prev: vec3f;
    if settings.frame >= 2 {
        prev = textureLoad(ping, vec2u(in.clip.xy), 0).xyz;
    } else {
        prev = vec3(0.0);
    }

    let next = cur + prev;
    textureStore(pong, vec2u(in.clip.xy), vec4(next, 0));

    return vec4(next / f32(settings.frame), 1);

    //return vec4f(in.vUv, 0.0, 1.0);
}