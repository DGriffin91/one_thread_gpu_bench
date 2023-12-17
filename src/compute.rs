use crate::{maybe_watch, timestamp::Timestamp, CompiledShaderModules, Options};

use std::{convert::TryInto, time::Instant};
use wgpu::{
    util::{
        backend_bits_from_env, dx12_shader_compiler_from_env,
        initialize_adapter_from_env_or_default, DeviceExt,
    },
    *,
};

pub fn start(options: &Options) {
    let compiled_shader_modules = maybe_watch(options, None);

    let gpu_result = futures::executor::block_on(start_internal(options, compiled_shader_modules));

    let start = Instant::now();
    let cpu_result = compute_shader::compute(options.size);
    let took = start.elapsed();
    println!("CPU Took: {took:?}");
    assert_eq!(gpu_result, cpu_result);
}

async fn start_internal(options: &Options, compiled_shader_modules: CompiledShaderModules) -> f32 {
    let backends = backend_bits_from_env().unwrap_or(Backends::PRIMARY);
    let instance = Instance::new(InstanceDescriptor {
        backends,
        dx12_shader_compiler: dx12_shader_compiler_from_env().unwrap_or_default(),
    });
    let adapter = initialize_adapter_from_env_or_default(&instance, backends, None)
        .await
        .expect("Failed to find an appropriate adapter");

    let mut features = Features::TIMESTAMP_QUERY | Features::TIMESTAMP_QUERY_INSIDE_PASSES;
    if options.force_spirv_passthru {
        features |= Features::SPIRV_SHADER_PASSTHROUGH;
    }

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

    let entry_point = "main_cs";

    // FIXME(eddyb) automate this decision by default.
    let module = compiled_shader_modules.spv_module_for_entry_point(entry_point);
    let module = if options.force_spirv_passthru {
        unsafe { device.create_shader_module_spirv(&module) }
    } else {
        let ShaderModuleDescriptorSpirV { label, source } = module;
        device.create_shader_module(ShaderModuleDescriptor {
            label,
            source: ShaderSource::SpirV(source),
        })
    };

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
        entry_point,
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

    println!("GPU Took: {:?}", timestamp.unmap(timestamp_slice));
    *result.first().unwrap()
}
