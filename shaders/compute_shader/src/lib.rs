#![cfg_attr(target_arch = "spirv", no_std)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
//#![deny(warnings)]

pub mod util;
use glam::*;
use spirv_std::arch::IndexUnchecked;
use spirv_std::{glam, spirv};

// Note: This cfg is incorrect on its surface, it really should be "are we compiling with std", but
// we tie #[no_std] above to the same condition, so it's fine.
#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;
use util::hash_noise;

pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}

impl Triangle {
    fn random(rng_coord: UVec2, seed: u32) -> Self {
        Triangle {
            a: vec3(
                hash_noise(rng_coord, 0 + seed),
                hash_noise(rng_coord, 1 + seed),
                hash_noise(rng_coord, 2 + seed),
            ) * 2.0
                - 1.0,
            b: vec3(
                hash_noise(rng_coord, 3 + seed),
                hash_noise(rng_coord, 4 + seed),
                hash_noise(rng_coord, 5 + seed),
            ) * 2.0
                - 1.0,
            c: vec3(
                hash_noise(rng_coord, 6 + seed),
                hash_noise(rng_coord, 7 + seed),
                hash_noise(rng_coord, 8 + seed),
            ) * 2.0
                - 1.0,
        }
    }

    pub fn intersect(&self, ray: Ray) -> Vec3 {
        let e1 = self.a - self.b;
        let e2 = self.c - self.a;
        let n = e1.cross(e2);

        let c = self.a - ray.origin;
        let r = ray.direction.cross(c);
        let inv_det = 1.0 / n.dot(ray.direction);

        let uvt = vec3(r.dot(e2), r.dot(e1), n.dot(c)) * inv_det;

        if (uvt.x > 0.0) & (uvt.y > 0.0) & (uvt.z > 0.0) & (uvt.x + uvt.y < 1.0) {
            uvt
        } else {
            vec3(f32::MAX, f32::MAX, f32::MAX)
        }
    }
}

pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    fn random(rng_coord: UVec2, seed: u32) -> Self {
        let origin = vec3(
            hash_noise(rng_coord, 0 + seed),
            hash_noise(rng_coord, 1 + seed),
            hash_noise(rng_coord, 2 + seed),
        ) * 2.0
            - 1.0;
        let direction = vec3(
            hash_noise(rng_coord, 3 + seed),
            hash_noise(rng_coord, 4 + seed),
            hash_noise(rng_coord, 5 + seed),
        ) * 2.0
            - 1.0;
        Ray {
            origin,
            direction: direction.normalize(),
        }
    }
}

//pub fn compute(settings: &UVec4) -> u32 {
//    let mut hash = settings.x as f32;
//    for_!((let mut x = 0; x < settings.y; x += 1) {
//        for_!((let mut y = 0; y < settings.z; y += 1) {
//            let coord = uvec2(x, y);
//            let a = vec3(
//                hash_noise(coord, 0),
//                hash_noise(coord, 1),
//                hash_noise(coord, 2),
//            );
//
//            hash += a.dot(vec3(1.0, 2.0, 3.0));
//        });
//    });
//
//    return hash as u32;
//}

//pub fn compute(settings: &UVec4) -> u32 {
//    let mut hash = settings.x as f32;
//    for x in 0..settings.y {
//        for y in 0..settings.z {
//            let coord = uvec2(x, y);
//            let tri = Triangle::random(coord, 0);
//            hash += tri.a.dot(tri.b) + tri.b.dot(tri.c) + tri.a.dot(tri.c);
//        }
//    }
//
//    return hash as u32;
//}

//pub fn compute(settings: &UVec4) -> u32 {
//    let mut hash = settings.x as f32;
//    for x in 0..settings.y {
//        for y in 0..settings.z {
//            let coord = uvec2(x, y);
//            let tri = Triangle::random(coord, 0);
//            let ray = Ray::random(coord, 9);
//            hash += tri.a.dot(ray.origin)
//                + tri.b.dot(ray.origin)
//                + tri.a.dot(ray.origin)
//                + tri.c.dot(ray.direction);
//        }
//    }
//
//    return hash as u32;
//}

pub fn compute(settings: &UVec4) -> u32 {
    let mut sum = settings.x as f32;
    for_!((let mut x = 0; x < settings.y; x += 1) {
        for_!((let mut y = 0; y < settings.z; y += 1) {
            let coord = uvec2(x, y);
            let tri = Triangle::random(coord, 0);
            let ray = Ray::random(coord, 9);
            sum += tri.intersect(ray).y.min(100.0).sin();
        });
    });

    return sum as u32;
}

//pub fn compute(settings: &UVec4) -> u32 {
//    let mut hash = settings.x as f32;
//    for_!((let mut x = 0; x < settings.y; x += 1) {
//        for_!((let mut y = 0; y < settings.z; y += 1) {
//            let coord = uvec2(x, y);
//
//            let a = vec3(
//                hash_noise(coord, 0),
//                hash_noise(coord, 1),
//                hash_noise(coord, 2),
//            ) * 2.0 - 1.0;
//            let b = vec3(
//                hash_noise(coord, 3),
//                hash_noise(coord, 4),
//                hash_noise(coord, 5),
//            ) * 2.0 - 1.0;
//            let c = vec3(
//                hash_noise(coord, 6),
//                hash_noise(coord, 7),
//                hash_noise(coord, 8),
//            ) * 2.0 - 1.0;
//
//            hash += a.dot(b) + b.dot(c) + a.dot(c);
//        });
//    });
//    return hash as u32;
//}

// Eq perf now
// pub fn compute(settings: &UVec4) -> u32 {
//     let mut hash = settings.x as f32;
//     for_!((let mut x = 0; x < settings.y; x += 1) {
//         for_!((let mut y = 0; y < settings.z; y += 1) {
//             let coord = uvec2(x, y);
//             hash += hash_noise(coord, 0);
//         });
//     });
//     return hash as u32;
// }

// Eq perf now
// pub fn compute(settings: &UVec4) -> u32 {
//     let mut hash = settings.x;
//     for_!((let mut i = 0; i < settings.y; i += 1) {
//         for_!((let mut j = 0; j < settings.z; j += 1) {
//             hash = hash * 1597334673;
//         });
//     });
//     return hash;
// }

// LocalSize/numthreads of (x = 1, y = 1, z = 1)
#[spirv(compute(threads(1)))]
pub fn main(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] data: &mut [u32],
    #[spirv(uniform, descriptor_set = 0, binding = 1)] settings: &UVec4,
) {
    let index = id.x as usize;
    unsafe {
        *data.index_unchecked_mut(index) = compute(settings);
    }
}
