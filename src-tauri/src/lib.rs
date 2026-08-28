mod runtime;

use serde::Serialize;
use std::process::Command;
use std::sync::Arc;
use sysinfo::System;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HardwareAccelerator {
    #[serde(rename = "type")]
    kind: String,
    name: String,
    memory_gb: Option<f64>,
    backend: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HardwareProfile {
    platform: String,
    architecture: String,
    chip: String,
    memory_gb: f64,
    os_version: String,
    preferred_backend: String,
    accelerators: Vec<HardwareAccelerator>,
    supports_metal: bool,
    supports_cuda: bool,
}

fn command_value(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn chip_name(system: &System) -> String {
    #[cfg(target_os = "macos")]
    {
        if let Some(value) = command_value("sysctl", &["-n", "machdep.cpu.brand_string"]) {
            return value;
        }
    }

    system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim().to_string())
        .filter(|brand| !brand.is_empty())
        .unwrap_or_else(|| "Unknown processor".to_string())
}

fn parse_nvidia_output(output: &str) -> Vec<HardwareAccelerator> {
    output
        .lines()
        .filter_map(|line| {
            let (name, memory_mib) = line.split_once(',')?;
            let memory_mib = memory_mib.trim().parse::<f64>().ok();
            Some(HardwareAccelerator {
                kind: "nvidia".to_string(),
                name: name.trim().to_string(),
                memory_gb: memory_mib.map(|mib| ((mib / 1024.0) * 10.0).round() / 10.0),
                backend: "cuda".to_string(),
            })
        })
        .filter(|accelerator| !accelerator.name.is_empty())
        .collect()
}

fn nvidia_accelerators() -> Vec<HardwareAccelerator> {
    command_value(
        "nvidia-smi",
        &[
            "--query-gpu=name,memory.total",
            "--format=csv,noheader,nounits",
        ],
    )
    .map(|output| parse_nvidia_output(&output))
    .unwrap_or_default()
}

#[tauri::command]
fn get_hardware_profile() -> HardwareProfile {
    let mut system = System::new_all();
    system.refresh_memory();

    let bytes = system.total_memory() as f64;
    let memory_gb = ((bytes / 1024_f64.powi(3)) * 10.0).round() / 10.0;
    let architecture = std::env::consts::ARCH.to_string();
    let platform = std::env::consts::OS.to_string();
    let chip = chip_name(&system);

    let supports_metal = cfg!(all(target_os = "macos", target_arch = "aarch64"));
    let mut accelerators = nvidia_accelerators();
    let supports_cuda = !accelerators.is_empty();

    if supports_metal {
        accelerators.insert(
            0,
            HardwareAccelerator {
                kind: "apple-unified".to_string(),
                name: format!("{} GPU", chip),
                memory_gb: Some(memory_gb),
                backend: "metal".to_string(),
            },
        );
    }

    let preferred_backend = if supports_metal {
        "Metal / MPS (production adapter validates availability at runtime)".to_string()
    } else if supports_cuda {
        "CUDA".to_string()
    } else {
        "CPU / Auto".to_string()
    };

    HardwareProfile {
        platform,
        architecture,
        chip,
        memory_gb,
        os_version: format!(
            "{} {}",
            System::name().unwrap_or_else(|| "Unknown OS".to_string()),
            System::os_version().unwrap_or_default()
        )
        .trim()
        .to_string(),
        preferred_backend,
        accelerators,
        supports_metal,
        supports_cuda,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(runtime::RuntimeState::default()))
        .invoke_handler(tauri::generate_handler![
            get_hardware_profile,
            runtime::get_model_runtime_states,
            runtime::install_model,
            runtime::uninstall_model,
            runtime::generate_triposr,
            runtime::cancel_generation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Still2Solid");
}

#[cfg(test)]
mod tests {
    use super::parse_nvidia_output;

    #[test]
    fn parses_nvidia_smi_memory_in_gib() {
        let accelerators = parse_nvidia_output("NVIDIA RTX 4090, 24564\n");
        assert_eq!(accelerators.len(), 1);
        assert_eq!(accelerators[0].name, "NVIDIA RTX 4090");
        assert_eq!(accelerators[0].memory_gb, Some(24.0));
    }
}
