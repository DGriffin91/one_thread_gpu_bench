use std::borrow::Cow;
use structopt::StructOpt;

mod compute;
mod timestamp;

struct CompiledShaderModules {
    named_spv_modules: Vec<(Option<String>, wgpu::ShaderModuleDescriptorSpirV<'static>)>,
}

impl CompiledShaderModules {
    fn spv_module_for_entry_point<'a>(
        &'a self,
        wanted_entry: &str,
    ) -> wgpu::ShaderModuleDescriptorSpirV<'a> {
        for (name, spv_module) in &self.named_spv_modules {
            match name {
                Some(name) if name != wanted_entry => continue,
                _ => {
                    return wgpu::ShaderModuleDescriptorSpirV {
                        label: name.as_deref(),
                        source: Cow::Borrowed(&spv_module.source),
                    };
                }
            }
        }
        unreachable!(
            "{wanted_entry:?} not found in modules {:?}",
            self.named_spv_modules
                .iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>()
        );
    }
}

fn maybe_watch(
    options: &Options,
    on_watch: Option<Box<dyn FnMut(CompiledShaderModules) + Send + 'static>>,
) -> CompiledShaderModules {
    use spirv_builder::{CompileResult, MetadataPrintout, SpirvBuilder};
    use std::path::PathBuf;

    let crate_path = [env!("CARGO_MANIFEST_DIR"), "shaders", "compute_shader"]
        .iter()
        .copied()
        .collect::<PathBuf>();

    let has_debug_printf = options.force_spirv_passthru;

    let builder = SpirvBuilder::new(crate_path, "spirv-unknown-vulkan1.1")
        .print_metadata(MetadataPrintout::None)
        .shader_panic_strategy(if has_debug_printf {
            spirv_builder::ShaderPanicStrategy::DebugPrintfThenExit {
                print_inputs: true,
                print_backtrace: true,
            }
        } else {
            spirv_builder::ShaderPanicStrategy::SilentExit
        })
        // HACK(eddyb) needed because of `debugPrintf` instrumentation limitations
        // (see https://github.com/KhronosGroup/SPIRV-Tools/issues/4892).
        .multimodule(has_debug_printf);
    let initial_result = if let Some(mut f) = on_watch {
        builder
            .watch(move |compile_result| f(handle_compile_result(compile_result)))
            .expect("Configuration is correct for watching")
    } else {
        builder.build().unwrap()
    };
    fn handle_compile_result(compile_result: CompileResult) -> CompiledShaderModules {
        let load_spv_module = |path| {
            let data = std::fs::read(path).unwrap();
            // FIXME(eddyb) this reallocates all the data pointlessly, there is
            // not a good reason to use `ShaderModuleDescriptorSpirV` specifically.
            let spirv = Cow::Owned(wgpu::util::make_spirv_raw(&data).into_owned());
            wgpu::ShaderModuleDescriptorSpirV {
                label: None,
                source: spirv,
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
    #[structopt(long)]
    force_spirv_passthru: bool,
    #[structopt(long, default_value = "512")]
    size: u32,
}

pub fn main() {
    std::env::set_var("WGPU_POWER_PREF", "high");

    let options: Options = Options::from_args();
    return compute::start(&options);
}
