@group(0) @binding(0) var<storage, read_write> data: array<u32>;
@group(0) @binding(1) var<uniform> settings: vec4<u32>;

const F32_MAX: f32 = 3.402823466E+38;

fn uhash(a: u32, b: u32) -> u32 { 
    var x = ((a * 1597334673u) ^ (b * 3812015801u));
    // from https://nullprogram.com/blog/2018/07/31/
    x = x ^ (x >> 16u);
    x = x * 0x7feb352du;
    x = x ^ (x >> 15u);
    x = x * 0x846ca68bu;
    x = x ^ (x >> 16u);
    return x;
}

fn unormf(n: u32) -> f32 { 
    return f32(n) * (1.0 / f32(0xffffffffu)); 
}

fn hash_noise(ufrag_coord: vec2<u32>, frame: u32) -> f32 {
    let urnd = uhash(ufrag_coord.x, (ufrag_coord.y << 11u) + frame);
    return unormf(urnd);
}

fn intersect(p0: vec3<f32>, p1: vec3<f32>, p2: vec3<f32>, origin: vec3<f32>, direction: vec3<f32>) -> vec3<f32> {
    let e1 = p0 - p1;
    let e2 = p2 - p0;
    let n = cross(e1, e2);
    
    let c = p0 - origin;
    let r = cross(direction, c);
    let inv_det = 1.0 / dot(n, direction);

    var uvt = vec3(
        dot(r, e2), 
        dot(r, e1),
        dot(n, c)
    ) * inv_det;

    //if all(uvt > vec3(0.0)) && uvt.x + uvt.y < 1.0 {
    if uvt.x > 0.0 && uvt.y > 0.0 && uvt.z > 0.0 && uvt.x + uvt.y < 1.0 {
        return uvt;
    }

    return vec3(F32_MAX);
}

//@compute @workgroup_size(1, 1, 1)
//fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
//    var hash = f32(settings.x);
//    for (var x = 0u; x < settings.y; x += 1u) {
//        for (var y = 0u; y < settings.z; y += 1u) {
//            let coord = vec2(x, y);
//            let a = vec3(
//                hash_noise(coord, 0u),
//                hash_noise(coord, 1u),
//                hash_noise(coord, 2u),
//            );
//            hash += dot(a, vec3(1.0,2.0,3.0));
//        }
//    }
//    data[invocation_id.x] = u32(hash);
//}

/*

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    var hash = f32(settings.x);
    for (var x = 0u; x < settings.y; x += 1u) {
        for (var y = 0u; y < settings.z; y += 1u) {
            let coord = vec2(x, y);

            let a = vec3(
                hash_noise(coord, 0u),
                hash_noise(coord, 1u),
                hash_noise(coord, 2u),
            ) * 2.0 - 1.0;
            let b = vec3(
                hash_noise(coord, 3u),
                hash_noise(coord, 4u),
                hash_noise(coord, 5u),
            ) * 2.0 - 1.0;
            let c = vec3(
                hash_noise(coord, 6u),
                hash_noise(coord, 7u),
                hash_noise(coord, 8u),
            ) * 2.0 - 1.0;
            let origin = vec3(
                hash_noise(coord, 9u),
                hash_noise(coord, 10u),
                hash_noise(coord, 11u),
            ) * 2.0 - 1.0;
            let direction = normalize(vec3(
                hash_noise(coord, 12u),
                hash_noise(coord, 13u),
                hash_noise(coord, 14u),
            ) * 2.0 - 1.0);

            hash += dot(a, origin) + dot(b, origin) + dot(a, origin) + dot(c, direction);
        }
    }
    data[invocation_id.x] = u32(hash);
}

*/


@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    var hash = f32(settings.x);
    for (var x = 0u; x < settings.y; x += 1u) {
        for (var y = 0u; y < settings.z; y += 1u) {
            let coord = vec2(x, y);

            let a = vec3(
                hash_noise(coord, 0u),
                hash_noise(coord, 1u),
                hash_noise(coord, 2u),
            ) * 2.0 - 1.0;
            let b = vec3(
                hash_noise(coord, 3u),
                hash_noise(coord, 4u),
                hash_noise(coord, 5u),
            ) * 2.0 - 1.0;
            let c = vec3(
                hash_noise(coord, 6u),
                hash_noise(coord, 7u),
                hash_noise(coord, 8u),
            ) * 2.0 - 1.0;
            let origin = vec3(
                hash_noise(coord, 9u),
                hash_noise(coord, 10u),
                hash_noise(coord, 11u),
            ) * 2.0 - 1.0;
            let direction = normalize(vec3(
                hash_noise(coord, 12u),
                hash_noise(coord, 13u),
                hash_noise(coord, 14u),
            ) * 2.0 - 1.0);

            hash += sin(min(intersect(a, b, c, origin, direction).y, 100.0));
        }
    }
    data[invocation_id.x] = u32(hash);
}


// Eq perf now
// @compute @workgroup_size(1, 1, 1)
// fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
//     var hash = f32(settings.x);
//     for (var x = 0u; x < settings.y; x += 1u) {
//         for (var y = 0u; y < settings.z; y += 1u) {
//             let coord = vec2(x, y);
//             hash += hash_noise(coord, 0u);
//         }
//     }
//     data[invocation_id.x] = u32(hash);
// }


//@compute @workgroup_size(1, 1, 1)
//fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
//    var hash = settings.x;
//    for (var i = 0u; i < settings.y; i += 1u) {
//        for (var j = 0u; j < settings.z; j += 1u) {
//            hash = (hash * 1597334673u);
//        }
//    }
//    data[invocation_id.x] = hash;
//}