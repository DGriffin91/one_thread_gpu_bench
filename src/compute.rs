use crate::{maybe_watch, timestamp::Timestamp, Options};

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

pub fn print_if_not_eq(a: f32, b: f32) {
    if a != b {
        println!("cpu != gpu: {} != {}", a, b)
    }
}

pub fn start(options: &Options) {
    let compiled_shader_modules = maybe_watch(None);

    let start = Instant::now();
    let cpu_result = compute_shader::compute(options.size);
    let took = start.elapsed();
    println!("CPU Took:\t{took:?}");

    let (gpu_duration, _gpu_result) = futures::executor::block_on(start_internal(
        options,
        compiled_shader_modules.named_spv_modules[0].1.clone(),
    ));
    println!("rust-gpu warm up Took:\t{:?}", gpu_duration);

    let (gpu_duration, gpu_result) = futures::executor::block_on(start_internal(
        options,
        compiled_shader_modules.named_spv_modules[0].1.clone(),
    ));
    println!("rust-gpu Took:\t{:?}", gpu_duration);
    print_if_not_eq(cpu_result, gpu_result);

    let (gpu_duration, gpu_result) = futures::executor::block_on(start_internal(
        options,
        include_wgsl!("compute_shader.wgsl"),
    ));
    println!("wgsl Took:\t{:?}", gpu_duration);
    print_if_not_eq(cpu_result, gpu_result);

    let src_path = [env!("CARGO_MANIFEST_DIR"), "src", "compute_shader.slang"]
        .iter()
        .copied()
        .collect::<PathBuf>();

    let dst_path = [
        env!("CARGO_MANIFEST_DIR"),
        "src",
        "compute_shader_slang.spv",
    ]
    .iter()
    .copied()
    .collect::<PathBuf>();
    let dst_string = dst_path.to_string_lossy().to_string();

    if options.compile_slang {
        let out = Command::new("slangc")
            .arg(src_path.to_string_lossy().to_string())
            //.arg("-O3")
            .arg("-profile")
            .arg("sm_5_0")
            .arg("-stage")
            .arg("compute")
            .arg("-entry")
            .arg("main")
            .arg("-o")
            .arg(dst_string.clone())
            .output()
            .expect("failed to execute process");
        if out.stderr.len() > 1 {
            println!("slangc stderr: {}", String::from_utf8_lossy(&out.stderr));
        }
    }

    let slang_spv = load_shader_module(&dst_path);

    let (gpu_duration, gpu_result) = futures::executor::block_on(start_internal(
        options,
        ShaderModuleDescriptor {
            label: Some(&dst_string),
            source: util::make_spirv(&slang_spv),
        },
    ));
    println!("slang Took:\t{:?}", gpu_duration);
    print_if_not_eq(cpu_result, gpu_result);
}

async fn start_internal(
    options: &Options,
    shader_module: ShaderModuleDescriptor<'_>,
) -> (Duration, f32) {
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
        .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
        .collect::<Vec<_>>();
    drop(data);
    readback_buffer.unmap();

    (timestamp.unmap(timestamp_slice), *result.first().unwrap())
}
