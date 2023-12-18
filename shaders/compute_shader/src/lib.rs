#![cfg_attr(target_arch = "spirv", no_std)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
//#![deny(warnings)]

pub mod util;
use glam::*;
use spirv_std::{glam, spirv};

// Note: This cfg is incorrect on its surface, it really should be "are we compiling with std", but
// we tie #[no_std] above to the same condition, so it's fine.
#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;
use util::hash_noise;

fn intersect(p0: Vec3, p1: Vec3, p2: Vec3, origin: Vec3, direction: Vec3) -> Vec3 {
    let e1 = p0 - p1;
    let e2 = p2 - p0;
    let n = e1.cross(e2);

    let c = p0 - origin;
    let r = direction.cross(c);
    let inv_det = 1.0 / n.dot(direction);

    let uvt = vec3(r.dot(e2), r.dot(e1), n.dot(c)) * inv_det;

    //if (uvt.x > 0.0) as u32
    //    & (uvt.y > 0.0) as u32
    //    & (uvt.z > 0.0) as u32
    //    & (uvt.x + uvt.y < 1.0) as u32
    //    == 1
    //{
    if uvt.x > 0.0 && uvt.y > 0.0 && uvt.z > 0.0 && uvt.x + uvt.y < 1.0 {
        uvt
    } else {
        vec3(f32::MAX, f32::MAX, f32::MAX)
    }
}

pub fn compute(size: u32) -> f32 {
    let mut sum = 0.0;
    for x in 0..size {
        for y in 0..size {
            let coord = uvec2(x, y);
            let a = vec3(
                hash_noise(coord, 0),
                hash_noise(coord, 1),
                hash_noise(coord, 2),
            ) * 2.0
                - 1.0;
            let b = vec3(
                hash_noise(coord, 3),
                hash_noise(coord, 4),
                hash_noise(coord, 5),
            ) * 2.0
                - 1.0;
            let c = vec3(
                hash_noise(coord, 6),
                hash_noise(coord, 7),
                hash_noise(coord, 8),
            ) * 2.0
                - 1.0;
            let origin = vec3(
                hash_noise(coord, 9),
                hash_noise(coord, 10),
                hash_noise(coord, 11),
            ) * 2.0
                - 1.0;
            let direction = vec3(
                hash_noise(coord, 12),
                hash_noise(coord, 13),
                hash_noise(coord, 14),
            ) * 2.0
                - 1.0;
            sum += intersect(a, b, c, origin, direction.normalize())
                .y
                .min(100.0)
                .sin();
        }
    }
    return sum;
}

// LocalSize/numthreads of (x = 1, y = 1, z = 1)
#[spirv(compute(threads(1)))]
pub fn main(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] data: &mut [f32],
    #[spirv(uniform, descriptor_set = 0, binding = 1)] settings: &UVec4,
) {
    let index = id.x as usize;
    data[index] = compute(settings.x);
}
