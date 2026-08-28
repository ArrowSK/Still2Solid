use serde::Serialize;
use std::process::Command;
use sysinfo::System;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HardwareProfile {
    platform: String,
    architecture: String,
    chip: String,
    memory_gb: f64,
    os_version: String,
    preferred_backend: String,
}

fn chip_name(system: &System) -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
        {
            if output.status.success() {
                let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !value.is_empty() {
                    return value;
                }
            }
        }
    }

    system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim().to_string())
        .filter(|brand| !brand.is_empty())
        .unwrap_or_else(|| "Unknown processor".to_string())
}

#[tauri::command]
fn get_hardware_profile() -> HardwareProfile {
    let mut system = System::new_all();
    system.refresh_memory();

    let bytes = system.total_memory() as f64;
    let memory_gb = ((bytes / 1024_f64.powi(3)) * 10.0).round() / 10.0;
    let architecture = std::env::consts::ARCH.to_string();
    let platform = std::env::consts::OS.to_string();
    let preferred_backend = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "Metal / MPS (when supported by model adapter)".to_string()
    } else {
        "Auto".to_string()
    };

    HardwareProfile {
        platform,
        architecture,
        chip: chip_name(&system),
        memory_gb,
        os_version: format!(
            "{} {}",
            System::name().unwrap_or_else(|| "Unknown OS".to_string()),
            System::os_version().unwrap_or_default()
        )
        .trim()
        .to_string(),
        preferred_backend,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_hardware_profile])
        .run(tauri::generate_context!())
        .expect("error while running Still2Solid");
}
