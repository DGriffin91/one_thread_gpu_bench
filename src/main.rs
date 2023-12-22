use std::{borrow::Cow, process::Command};
use structopt::StructOpt;
use wgpu::*;

mod compute;
mod timestamp;

struct CompiledShaderModules {
    named_spv_modules: Vec<(Option<String>, ShaderModuleDescriptor<'static>)>,
}

fn maybe_watch(
    on_watch: Option<Box<dyn FnMut(CompiledShaderModules) + Send + 'static>>,
) -> CompiledShaderModules {
    use spirv_builder::{CompileResult, MetadataPrintout, SpirvBuilder};
    use std::path::PathBuf;

    std::env::set_var(
        "RUSTGPU_CODEGEN_ARGS",
        "--dump-spirt-passes=$PWD/spirt-passes --spirt-passes=reduce,fuse_selects",
    );

    let crate_path = [env!("CARGO_MANIFEST_DIR"), "shaders", "compute_shader"]
        .iter()
        .copied()
        .collect::<PathBuf>();

    let builder = SpirvBuilder::new(crate_path, "spirv-unknown-vulkan1.1")
        .print_metadata(MetadataPrintout::None)
        .shader_panic_strategy(spirv_builder::ShaderPanicStrategy::SilentExit);
    let initial_result = if let Some(mut f) = on_watch {
        builder
            .watch(move |compile_result| f(handle_compile_result(compile_result)))
            .expect("Configuration is correct for watching")
    } else {
        builder.build().unwrap()
    };
    fn handle_compile_result(compile_result: CompileResult) -> CompiledShaderModules {
        let spv_path = [
            env!("CARGO_MANIFEST_DIR"),
            "shaders",
            "compute_shader_rust.spv",
        ]
        .iter()
        .copied()
        .collect::<PathBuf>();
        let glsl_path = spv_path.with_extension("glsl");
        let _ = std::fs::remove_file(&glsl_path);
        let mut cmd = Command::new("spirv-cross");
        cmd.arg(&spv_path).arg("--output").arg(&glsl_path);
        let out = cmd.output().expect("failed to execute process");
        if out.stderr.len() > 1 {
            println!(
                "spirv-cross stderr: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Command::new("spv-lower-print")
            .arg(&spv_path)
            .output()
            .expect("failed to execute process");

        std::fs::copy(compile_result.module.unwrap_single(), spv_path).unwrap();
        let load_spv_module = |path| {
            let data = std::fs::read(path).unwrap();
            let spirv = Cow::Owned(util::make_spirv_raw(&data).into_owned());
            ShaderModuleDescriptor {
                label: None,
                source: ShaderSource::SpirV(spirv),
            }
        };
        CompiledShaderModules {
            named_spv_modules: match compile_result.module {
                spirv_builder::ModuleResult::SingleModule(path) => {
                    vec![(None, load_spv_module(path))]
                }
                spirv_builder::ModuleResult::MultiModule(modules) => modules
                    .into_iter()
                    .map(|(name, path)| (Some(name), load_spv_module(path)))
                    .collect(),
            },
        }
    }
    handle_compile_result(initial_result)
}

#[derive(StructOpt, Clone)]
#[structopt(name = "example-runner-wgpu")]
pub struct Options {
    #[structopt(long, default_value = "512")]
    size: u32,
    #[structopt(long)]
    compile_slang: bool,
}

pub fn main() {
    std::env::set_var("WGPU_POWER_PREF", "high");

    let options: Options = Options::from_args();
    return compute::start(&options);
}
