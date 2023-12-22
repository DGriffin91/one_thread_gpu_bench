use core::{f32::consts::PI, num::Wrapping};

#[cfg(target_arch = "spirv")]
use spirv_std::arch::{signed_max, signed_min, unsigned_max, unsigned_min};

use spirv_std::glam::*;

#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;

pub trait GpuMinMax {
    fn minv(self, other: Self) -> Self;
    fn maxv(self, other: Self) -> Self;
}

impl GpuMinMax for i32 {
    fn minv(self, other: Self) -> Self {
        #[cfg(target_arch = "spirv")]
        {
            signed_min(self, other)
        }
        #[cfg(not(target_arch = "spirv"))]
        {
            self.min(other)
        }
    }
    fn maxv(self, other: Self) -> Self {
        #[cfg(target_arch = "spirv")]
        {
            signed_max(self, other)
        }
        #[cfg(not(target_arch = "spirv"))]
        {
            self.max(other)
        }
    }
}

impl GpuMinMax for u32 {
    fn minv(self, other: Self) -> Self {
        #[cfg(target_arch = "spirv")]
        {
            unsigned_min(self, other)
        }
        #[cfg(not(target_arch = "spirv"))]
        {
            self.min(other)
        }
    }
    fn maxv(self, other: Self) -> Self {
        #[cfg(target_arch = "spirv")]
        {
            unsigned_max(self, other)
        }
        #[cfg(not(target_arch = "spirv"))]
        {
            self.max(other)
        }
    }
}

pub trait F32ScalarSwizzle {
    fn xxxx(self) -> Vec4;
    fn xxx(self) -> Vec3;
    fn xx(self) -> Vec2;
}

impl F32ScalarSwizzle for f32 {
    fn xxxx(self) -> Vec4 {
        Vec4::splat(self)
    }
    fn xxx(self) -> Vec3 {
        Vec3::splat(self)
    }
    fn xx(self) -> Vec2 {
        Vec2::splat(self)
    }
}

pub trait U32ScalarSwizzle {
    fn xxxx(self) -> UVec4;
    fn xxx(self) -> UVec3;
    fn xx(self) -> UVec2;
}

impl U32ScalarSwizzle for u32 {
    fn xxxx(self) -> UVec4 {
        UVec4::splat(self)
    }
    fn xxx(self) -> UVec3 {
        UVec3::splat(self)
    }
    fn xx(self) -> UVec2 {
        UVec2::splat(self)
    }
}

pub trait I32ScalarSwizzle {
    fn xxxx(self) -> IVec4;
    fn xxx(self) -> IVec3;
    fn xx(self) -> IVec2;
}

impl I32ScalarSwizzle for i32 {
    fn xxxx(self) -> IVec4 {
        IVec4::splat(self)
    }
    fn xxx(self) -> IVec3 {
        IVec3::splat(self)
    }
    fn xx(self) -> IVec2 {
        IVec2::splat(self)
    }
}

pub fn saturate(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

pub fn pow(v: Vec3, power: f32) -> Vec3 {
    vec3(v.x.powf(power), v.y.powf(power), v.z.powf(power))
}

pub fn exp(v: Vec3) -> Vec3 {
    vec3(v.x.exp(), v.y.exp(), v.z.exp())
}

/// Based on: <https://seblagarde.wordpress.com/2014/12/01/inverse-trigonometric-functions-gpu-optimization-for-amd-gcn-architecture/>
pub fn acos_approx(v: f32) -> f32 {
    let x = v.abs();
    let mut res = -0.155972 * x + 1.56467; // p(x)
    res *= (1.0f32 - x).sqrt();

    if v >= 0.0 {
        res
    } else {
        PI - res
    }
}

pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    // Scale, bias and saturate x to 0..1 range
    let x = saturate((x - edge0) / (edge1 - edge0));
    // Evaluate polynomial
    x * x * (3.0 - 2.0 * x)
}

pub fn uhash(a: u32, b: u32) -> u32 {
    let mut x =
        (Wrapping(a) * Wrapping(1597334673u32)).0 ^ (Wrapping(b) * Wrapping(3812015801u32)).0;
    // from https://nullprogram.com/blog/2018/07/31/
    x = x ^ (x >> 16u32);
    x = (Wrapping(x) * Wrapping(0x7feb352du32)).0;
    x = x ^ (x >> 15u32);
    x = (Wrapping(x) * Wrapping(0x846ca68bu32)).0;
    x = x ^ (x >> 16u32);
    x
}

//pub fn uhash(a: u32, b: u32) -> u32 {
//    let mut x = (a * 1597334673u32) ^ (b * 3812015801u32);
//    // from https://nullprogram.com/blog/2018/07/31/
//    x = x ^ (x >> 16u32);
//    x = x * 0x7feb352du32;
//    x = x ^ (x >> 15u32);
//    x = x * 0x846ca68bu32;
//    x = x ^ (x >> 16u32);
//    x
//}

pub fn unormf(n: u32) -> f32 {
    n as f32 * (1.0 / 0xffffffffu32 as f32)
}

pub fn hash_noise(ucoord: UVec2, frame: u32) -> f32 {
    let urnd = uhash(ucoord.x, (ucoord.y << 11u32) + frame);
    unormf(urnd)
}

#[macro_export]
macro_rules! for_ {
    (($start:stmt; $cond:expr; $inc:expr) { $($body:tt)* }) => {{
        $start
        if ($cond) {
            loop {
                $($body)*
                $inc;
                if !($cond) {
                    break;
                }
            }
        }
    }};
}
