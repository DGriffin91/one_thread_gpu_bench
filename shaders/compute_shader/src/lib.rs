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
            return uvt;
        }

        return vec3(f32::MAX, f32::MAX, f32::MAX);
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

pub fn compute(size: u32) -> f32 {
    let mut sum = 0.0;
    //for x in 0..size {
    //    for y in 0..size {
    //        let coord = uvec2(x, y);
    //        let tri = Triangle::random(coord, 0);
    //        let ray = Ray::random(coord, 9);
    //        sum += tri.intersect(ray).y.min(100.0).sin();
    //    }
    //}

    for_!((let mut x = 0; x < size; x += 1) {
        for_!((let mut y = 0; y < size; y += 1) {
            let coord = uvec2(x, y);
            let tri = Triangle::random(coord, 0);
            let ray = Ray::random(coord, 9);
            sum += tri.intersect(ray).y.min(100.0).sin();
        });
    });

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
