[[vk::binding(0, 0)]]
RWStructuredBuffer<uint> data : register(u0);
[[vk::binding(1, 0)]]
cbuffer SettingsBuffer : register(b1)
{
    uint4 settings;
};

#define F32_MAX 3.402823466E+38

// HLSL functions equivalent to WGSL
uint uhash(uint a, uint b)
{
    var x = ((a * 1597334673u) ^ (b * 3812015801u));
    // from https://nullprogram.com/blog/2018/07/31/
    x = x ^ (x >> 16u);
    x = x * 0x7feb352du;
    x = x ^ (x >> 15u);
    x = x * 0x846ca68bu;
    x = x ^ (x >> 16u);
    return x;
}

float unormf(uint n)
{
    return float(n) * (1.0 / float(0xffffffffu));
}

float hash_noise(uint2 ufrag_coord, uint frame)
{
    let urnd = uhash(ufrag_coord.x, (ufrag_coord.y << 11u) + frame);
    return unormf(urnd);
}

struct Ray
{
    float3 origin;
    float3 direction;

    static Ray random(uint2 coord, uint seed)
    {
        var ray : Ray;
        ray.origin = float3(
                         hash_noise(coord, seed + 0u),
                         hash_noise(coord, seed + 1u),
                         hash_noise(coord, seed + 2u)) *
                         2.0 -
                     1.0;
        ray.direction = float3(
                            hash_noise(coord, seed + 3u),
                            hash_noise(coord, seed + 4u),
                            hash_noise(coord, seed + 5u)) *
                            2.0 -
                        1.0;
        ray.direction = normalize(ray.direction);
        return ray;
    }
}

struct Triangle
{
    float3 a;
    float3 b;
    float3 c;

    float3 intersect(Ray ray)
    {
        let e1 = a - b;
        let e2 = c - a;
        let n = cross(e1, e2);

        let c = a - ray.origin;
        let r = cross(ray.direction, c);
        let inv_det = 1.0 / dot(n, ray.direction);

        let uvt = float3(
                      dot(r, e2),
                      dot(r, e1),
                      dot(n, c)) *
                  inv_det;

        if (uvt.x > 0.0 && uvt.y > 0.0 && uvt.z > 0.0 && uvt.x + uvt.y < 1.0)
        {
            return uvt;
        }

        return float3(F32_MAX);
    }

    static Triangle random(uint2 coord, uint seed)
    {
        var tri : Triangle;
        tri.a = float3(
                    hash_noise(coord, seed + 0u),
                    hash_noise(coord, seed + 1u),
                    hash_noise(coord, seed + 2u)) *
                    2.0 -
                1.0;
        tri.b = float3(
                    hash_noise(coord, seed + 3u),
                    hash_noise(coord, seed + 4u),
                    hash_noise(coord, seed + 5u)) *
                    2.0 -
                1.0;
        tri.c = float3(
                    hash_noise(coord, seed + 6u),
                    hash_noise(coord, seed + 7u),
                    hash_noise(coord, seed + 8u)) *
                    2.0 -
                1.0;
        return tri;
    }
}

[numthreads(1, 1, 1)]
void main(uint3 invocation_id: SV_DispatchThreadID)
{

    var hash = (float)settings.x;
    for (uint x = 0u; x < settings.y; x++)
    {
        for (uint y = 0u; y < settings.z; y++)
        {
            uint2 coord = uint2(x, y);
            let tri = Triangle::random(coord, 0u);
            let ray = Ray::random(coord, 9u);
            hash += sin(min(tri.intersect(ray).y, 100.0));
        }
    }

    uint index = invocation_id.x;
    data[index] = (uint)hash;
}

//[numthreads(1, 1, 1)]
// void main(uint3 invocation_id: SV_DispatchThreadID)
//{
//    float hash = (float)settings.x;
//    for (uint x = 0u; x < settings.y; x++)
//    {
//        for (uint y = 0u; y < settings.z; y++)
//        {
//            uint2 coord = uint2(x, y);
//            let a = float3(
//                hash_noise(coord, 0),
//                hash_noise(coord, 1),
//                hash_noise(coord, 2));
//            hash += dot(a, float3(1.0, 2.0, 3.0));
//        }
//    }
//    data[invocation_id.x] = (uint)hash;
//}

/*

[numthreads(1, 1, 1)]
void main(uint3 invocation_id: SV_DispatchThreadID)
{
    float hash = (float)settings.x;
    for (uint x = 0u; x < settings.y; x++)
    {
        for (uint y = 0u; y < settings.z; y++)
        {
            uint2 coord = uint2(x, y);
            let tri = Triangle::random(coord, 0u);
            let ray = Ray::random(coord, 9u);
            hash += dot(tri.a, ray.origin) + dot(tri.b, ray.origin) + dot(tri.a, ray.origin) + dot(tri.c, ray.direction);
        }
    }
    data[invocation_id.x] = (uint)hash;
}

*/

// Eq perf now
// [numthreads(1, 1, 1)]
// void main(uint3 invocation_id: SV_DispatchThreadID)
// {
//     float hash = (float)settings.x;
//     for (uint x = 0u; x < settings.y; x++)
//     {
//         for (uint y = 0u; y < settings.z; y++)
//         {
//             uint2 coord = uint2(x, y);
//             hash += hash_noise(coord, 0);
//         }
//     }
//     data[invocation_id.x] = (uint)hash;
// }

//[numthreads(1, 1, 1)]
// void main(uint3 invocation_id: SV_DispatchThreadID)
//{
//    uint hash = settings.x;
//    for (uint i = 0u; i < settings.y; i++)
//    {
//        for (uint j = 0u; j < settings.z; j++)
//        {
//            hash = (hash * 1597334673);
//        }
//    }
//    data[invocation_id.x] = hash;
//}