use crate::{maybe_watch, timestamp::Timestamp, Options};
use spirv_std::glam::UVec4;

use std::{
    convert::TryInto,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};
use wgpu::{
    util::{
        backend_bits_from_env, dx12_shader_compiler_from_env,
        initialize_adapter_from_env_or_default, DeviceExt,
    },
    *,
};

fn load_shader_module(path: &Path) -> Vec<u8> {
    let mut f = std::fs::File::open(&path).expect("no file found");
    let metadata = std::fs::metadata(&path).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");
    buffer
}

pub fn print_if_not_eq(a: u32, b: u32) {
    if a != b {
        println!("cpu != gpu: {} != {}", a, b)
    }
}

pub fn start(options: &Options) {
    let compiled_shader_modules = maybe_watch(None);

    let start = Instant::now();
    let cpu_result = compute_shader::compute(&UVec4::splat(options.size));
    println!("{}\trust cpu", to_ms_round(start.elapsed()));

    let (gpu_duration, gpu_result) = futures::executor::block_on(start_internal(
        options,
        compiled_shader_modules.named_spv_modules[0].1.clone(),
    ));
    println!("{}\trust gpu", to_ms_round(gpu_duration));
    print_if_not_eq(cpu_result, gpu_result);

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    let rust_spv = src.join("compute_shader_rust.spv");
    let rust_spv_opt = src.join("compute_shader_rust.opt.spv");
    spirv_opt(&rust_spv, &rust_spv_opt);
    run_spv(&rust_spv_opt, options, cpu_result, "rust > opt");

    let wgsl = src.join("compute_shader.wgsl");
    let wgsl_spv = src.join("compute_shader.wgsl.spv");
    let wgsl_spv_opt = src.join("compute_shader.wgsl.opt.spv");

    naga(&wgsl, &wgsl_spv);
    run_spv(&wgsl_spv, options, cpu_result, "wgsl");
    spirv_opt(&wgsl_spv, &wgsl_spv_opt);
    run_spv(&wgsl_spv_opt, options, cpu_result, "wgsl > spirv-opt");

    generate_spirt(&wgsl_spv_opt);
    let wgsl_spv_opt_link = src.join("compute_shader.wgsl.opt.link.spv");
    //spv_lower_print(&wgsl_spv_opt_link);
    run_spv(
        &wgsl_spv_opt_link,
        options,
        cpu_result,
        "wgsl > spirv-opt > spirt",
    );

    let src_path = [env!("CARGO_MANIFEST_DIR"), "src", "compute_shader.slang"]
        .iter()
        .copied()
        .collect::<PathBuf>();

    let slang_spv_path = [
        env!("CARGO_MANIFEST_DIR"),
        "src",
        "compute_shader_slang.spv",
    ]
    .iter()
    .copied()
    .collect::<PathBuf>();

    slangc(src_path, &slang_spv_path);

    run_spv(&slang_spv_path, options, cpu_result, "slang");
    spv_lower_print(&slang_spv_path);

    let slang_spv_opt_path = slang_spv_path.with_file_name("compute_shader_slang.opt.spv");

    spirv_opt(&slang_spv_path, &slang_spv_opt_path);

    run_spv(
        &slang_spv_opt_path,
        options,
        cpu_result,
        "slang > spirv-opt",
    );

    spv_lower_print(&slang_spv_opt_path);

    generate_spirt(&slang_spv_opt_path);

    let slang_naga_link_spv_path =
        slang_spv_path.with_file_name("compute_shader_slang.opt.link.spv");

    run_spv(
        &slang_naga_link_spv_path,
        options,
        cpu_result,
        "slang > spirv-opt > spirt",
    );
}

fn slangc(src_path: PathBuf, slang_spv_path: &PathBuf) {
    let out = Command::new("slangc")
        .arg(src_path)
        .arg("-O3")
        .arg("-profile")
        .arg("sm_5_0")
        .arg("-stage")
        .arg("compute")
        .arg("-entry")
        .arg("main")
        .arg("-o")
        .arg(slang_spv_path)
        .output()
        .expect("failed to execute process");
    if out.stderr.len() > 1 {
        println!("slangc stderr: {}", String::from_utf8_lossy(&out.stderr));
    }
}

fn spirv_opt(input: &Path, output: &Path) {
    let out = Command::new("spirv-opt")
        .arg("-O")
        .arg(&input)
        .arg("-o")
        .arg(output)
        .output()
        .expect("failed to execute process");
    if out.stderr.len() > 1 {
        println!("spirv-opt stderr: {}", String::from_utf8_lossy(&out.stderr));
    }
}

fn generate_spirt(path: &Path) {
    let out = Command::new("spv-lower-link-lift")
        .args([&path])
        .output()
        .expect("failed to execute process");
    if out.stderr.len() > 1 {
        // Noisy
        //println!(
        //    "spv-lower-link-lift stderr: {}",
        //    String::from_utf8_lossy(&out.stderr)
        //);
    }
}

fn naga(input: &Path, output: &Path) {
    let out = Command::new("naga")
        .args([input, output])
        .output()
        .expect("failed to execute process");
    if out.stderr.len() > 1 {
        println!("naga stderr: {}", String::from_utf8_lossy(&out.stderr));
    }
}

fn spv_lower_print(path: &Path) {
    let out = Command::new("spv-lower-print")
        .arg(&path)
        .output()
        .expect("failed to execute process");
    if out.stderr.len() > 1 {
        println!("spv-lower-print: {}", String::from_utf8_lossy(&out.stderr));
    }
}

fn run_spv(
    slang_spv_path: &Path,
    options: &Options,
    cpu_result: u32,
    text: &str,
) -> (Duration, u32) {
    let slang_spv = load_shader_module(slang_spv_path);
    let (gpu_duration, gpu_result) = futures::executor::block_on(start_internal(
        options,
        ShaderModuleDescriptor {
            label: None,
            source: util::make_spirv(&slang_spv),
        },
    ));
    println!("{}\t{}", to_ms_round(gpu_duration), text);
    print_if_not_eq(cpu_result, gpu_result);
    (gpu_duration, gpu_result)
}

fn to_ms_round(d: Duration) -> String {
    format!("{:.1}ms", d.as_secs_f32() * 1000.0)
}

async fn start_internal(
    options: &Options,
    shader_module: ShaderModuleDescriptor<'_>,
) -> (Duration, u32) {
    let backends = backend_bits_from_env().unwrap_or(Backends::PRIMARY);
    let instance = Instance::new(InstanceDescriptor {
        backends,
        dx12_shader_compiler: dx12_shader_compiler_from_env().unwrap_or_default(),
    });
    let adapter = initialize_adapter_from_env_or_default(&instance, backends, None)
        .await
        .expect("Failed to find an appropriate adapter");

    let features = Features::TIMESTAMP_QUERY | Features::TIMESTAMP_QUERY_INSIDE_PASSES;

    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: None,
                features,
                limits: Limits::default(),
            },
            None,
        )
        .await
        .expect("Failed to create device");
    drop(instance);
    drop(adapter);

    let module = device.create_shader_module(shader_module);

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                count: None,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    has_dynamic_offset: false,
                    min_binding_size: None,
                    ty: BufferBindingType::Storage { read_only: false },
                },
            },
            BindGroupLayoutEntry {
                binding: 1,
                count: None,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    has_dynamic_offset: false,
                    min_binding_size: None,
                    ty: BufferBindingType::Uniform,
                },
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: "main",
    });

    let storage_buffer_size = 4;

    let readback_buffer = device.create_buffer(&BufferDescriptor {
        label: None,
        size: storage_buffer_size,
        // Can be read to the CPU, and can be copied from the shader's storage buffer
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let storage_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
        label: Some("Bench Input"),
        contents: &[0_u8; 4],
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
    });

    let uniform_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
        label: Some("Bench Uniform"),
        contents: &bytemuck::bytes_of(&[options.size; 4]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let timestamp = Timestamp::new(&device, &queue);

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: storage_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

    {
        let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor { label: None });
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.set_pipeline(&compute_pipeline);
        // Warm up
        cpass.dispatch_workgroups(1, 1, 1);
        cpass.dispatch_workgroups(1, 1, 1);
        cpass.dispatch_workgroups(1, 1, 1);
        cpass.dispatch_workgroups(1, 1, 1);
        // Start bench
        timestamp.start(&mut cpass);
        cpass.dispatch_workgroups(1, 1, 1);
        timestamp.end(&mut cpass);
    }

    encoder.copy_buffer_to_buffer(&storage_buffer, 0, &readback_buffer, 0, storage_buffer_size);
    timestamp.resolve(&mut encoder);

    queue.submit(Some(encoder.finish()));
    let buffer_slice = readback_buffer.slice(..);
    let timestamp_slice = timestamp.map();
    buffer_slice.map_async(MapMode::Read, |r| r.unwrap());
    // NOTE(eddyb) `poll` should return only after the above callbacks fire
    // (see also https://github.com/gfx-rs/wgpu/pull/2698 for more details).
    device.poll(Maintain::Wait);

    let data = buffer_slice.get_mapped_range();
    let result = data
        .chunks_exact(4)
        .map(|b| u32::from_ne_bytes(b.try_into().unwrap()))
        .collect::<Vec<_>>();
    drop(data);
    readback_buffer.unmap();

    (timestamp.unmap(timestamp_slice), *result.first().unwrap())
}
