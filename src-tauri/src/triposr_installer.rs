use crate::runtime::{get_model_runtime_states, ModelRuntimeState};
use md5;
use reqwest::blocking::Client;
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::{Digest as Sha2Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

const TRIPOSR_SOURCE_REVISION: &str = "107cefdc244c39106fa830359024f6a2f1c78871";
const TRIPOSR_WEIGHT_REVISION: &str = "5b521936b01fbe1890f6f9baed0254ab6351c04a";
const TRIPOSR_WEIGHT_SHA256: &str = "429e2c6b22a0923967459de24d67f05962b235f79cde6b032aa7ed2ffcd970ee";
const U2NET_MD5: &str = "60024c5c889badc19c04ad937298a77b";
const INSTALL_SCHEMA: u32 = 1;

const WORKER: &str = include_str!("../../workers/triposr/worker.py");
const TRIPOSR_CONFIG: &str = include_str!("../../workers/triposr/config.yaml");
const DINO_CONFIG: &str = include_str!("../../workers/triposr/dino-config.json");

// Exact files that exist at TRIPOSR_SOURCE_REVISION. Keep the Git blob SHA-1 next
// to each path so an upstream path change or unexpected response cannot be activated.
pub(crate) const SOURCE_FILES: &[(&str, &str)] = &[
    ("LICENSE", "6c440ca1ef32cedcc2257b99953add129199ed26"),
    ("tsr/system.py", "bcdb69b2e5c85b3c0ffebd7ff82fc4fe7b3d543e"),
    ("tsr/utils.py", "9758f2e44cd4091f2124929edc89b59e98fff667"),
    ("tsr/bake_texture.py", "642a4242e7952999e3a329d9a3fc7995dd3f86b0"),
    ("tsr/models/isosurface.py", "076321b81f573ab95a6c7d5328f6366e0f33c38e"),
    ("tsr/models/nerf_renderer.py", "0cf3ab6e31f79c3af8e75384f48c0e874621c2b8"),
    ("tsr/models/network_utils.py", "470c92b9a99901e93dee2b98e80eea53cfdf3f9d"),
    ("tsr/models/tokenizers/image.py", "d4034b25a7b2e8f38630fd69a0bae75ed2a5edb0"),
    ("tsr/models/tokenizers/triplane.py", "ecdd7fd2201c974bb70b18a90a633287b814886f"),
    ("tsr/models/transformer/attention.py", "34bd99778ce168cb68192c183814f098a3d08e67"),
    ("tsr/models/transformer/basic_transformer_block.py", "1ea41b0192b6fbb368fe3a587b7c04b0253d23b3"),
    ("tsr/models/transformer/transformer_1d.py", "a98d0fb98136ea873aef708717a809e434e7ca14"),
];

const PYTHON_REQUIREMENTS: &[&str] = &[
    "torch==2.13.0",
    "torchvision==0.28.0",
    "omegaconf==2.3.0",
    "Pillow==12.1.0",
    "einops==0.8.1",
    "transformers==4.35.0",
    "trimesh==4.0.5",
    "rembg==2.0.77",
    "imageio==2.37.0",
    "xatlas==0.0.11",
    "moderngl==5.10.0",
    "scikit-image==0.26.0",
];

#[derive(Default)]
pub struct TripoInstallerState {
    lock: Mutex<()>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallProgressEvent {
    model_id: String,
    stage: String,
    stage_progress: f64,
    overall_progress: f64,
    message: String,
    bytes_downloaded: Option<u64>,
    bytes_total: Option<u64>,
}

fn model_root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("models")
        .join("triposr"))
}

fn runtime_python(root: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        root.join("runtime").join("Scripts").join("python.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        root.join("runtime").join("bin").join("python")
    }
}

fn find_python(app: &AppHandle) -> Result<(String, String), String> {
    let mut candidates = Vec::<String>::new();
    if let Ok(resources) = app.path().resource_dir() {
        #[cfg(target_os = "windows")]
        let executable = "python.exe";
        #[cfg(not(target_os = "windows"))]
        let executable = "bin/python3";
        for base in [resources.join("python"), resources.join("resources").join("python")] {
            let path = base.join(executable);
            if path.exists() {
                candidates.push(path.to_string_lossy().to_string());
            }
        }
    }
    if let Ok(explicit) = std::env::var("STILL2SOLID_PYTHON") {
        if !explicit.trim().is_empty() && !candidates.iter().any(|candidate| candidate == &explicit) {
            candidates.push(explicit);
        }
    }
    #[cfg(target_os = "windows")]
    let system_candidates = ["python.exe", "python3.12.exe", "python3.11.exe"];
    #[cfg(not(target_os = "windows"))]
    let system_candidates = ["python3.12", "python3.11", "python3"];
    for candidate in system_candidates {
        if !candidates.iter().any(|value| value == candidate) {
            candidates.push(candidate.to_string());
        }
    }

    for candidate in candidates {
        let output = Command::new(&candidate)
            .args([
                "-c",
                "import sys; print(f'{sys.version_info.major}|{sys.version_info.minor}|{sys.version.split()[0]}')",
            ])
            .output();
        let Ok(output) = output else { continue };
        if !output.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut parts = text.trim().split('|');
        let major = parts.next().and_then(|value| value.parse::<u32>().ok());
        let minor = parts.next().and_then(|value| value.parse::<u32>().ok());
        let version = parts.next().map(ToOwned::to_owned);
        if major == Some(3) && matches!(minor, Some(11 | 12)) {
            if let Some(version) = version {
                return Ok((candidate, version));
            }
        }
    }

    Err("Still2Solid could not find its bundled Python 3.12 runtime. Reinstall Still2Solid if this is a release build.".to_string())
}

fn emit_install(
    app: &AppHandle,
    stage: &str,
    stage_progress: f64,
    overall_progress: f64,
    message: impl Into<String>,
    bytes_downloaded: Option<u64>,
    bytes_total: Option<u64>,
) {
    let _ = app.emit(
        "model-install-progress",
        InstallProgressEvent {
            model_id: "triposr".into(),
            stage: stage.into(),
            stage_progress,
            overall_progress,
            message: message.into(),
            bytes_downloaded,
            bytes_total,
        },
    );
}

fn download(
    app: &AppHandle,
    client: &Client,
    url: &str,
    destination: &Path,
    stage: &str,
    overall_start: f64,
    overall_end: f64,
    message: &str,
) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = destination.with_extension("part");
    let mut response = client
        .get(url)
        .send()
        .map_err(|error| format!("Download failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Download failed with HTTP {}: {url}", response.status()));
    }
    let total = response.content_length();
    let mut file = File::create(&temporary).map_err(|error| error.to_string())?;
    let mut downloaded = 0u64;
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let read = response.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read]).map_err(|error| error.to_string())?;
        downloaded += read as u64;
        let progress = total
            .filter(|value| *value > 0)
            .map(|value| (downloaded as f64 / value as f64).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        emit_install(
            app,
            stage,
            progress,
            overall_start + (overall_end - overall_start) * progress,
            message,
            Some(downloaded),
            total,
        );
    }
    file.flush().map_err(|error| error.to_string())?;
    fs::rename(&temporary, destination).map_err(|error| error.to_string())?;
    Ok(())
}

fn git_blob_sha1(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha1::new();
    hasher.update(format!("blob {}\0", bytes.len()).as_bytes());
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn md5_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", md5::compute(bytes)))
}

fn install_blocking(app: AppHandle) -> Result<ModelRuntimeState, String> {
    let final_root = model_root(&app)?;
    let staging = final_root.with_extension("installing");
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&staging).map_err(|error| error.to_string())?;

    let result = (|| -> Result<(), String> {
        emit_install(&app, "runtime", 0.0, 0.01, "Finding Still2Solid's Python runtime", None, None);
        let (system_python, python_version) = find_python(&app)?;
        let runtime = staging.join("runtime");
        emit_install(&app, "runtime", 0.1, 0.02, "Creating the private TripoSR runtime", None, None);
        let status = Command::new(&system_python)
            .args(["-m", "venv"])
            .arg(&runtime)
            .status()
            .map_err(|error| error.to_string())?;
        if !status.success() {
            return Err("Python could not create the private TripoSR environment.".into());
        }

        let python = runtime_python(&staging);
        let mut pip_args = vec![
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--no-input",
            "--no-cache-dir",
        ];
        pip_args.extend(PYTHON_REQUIREMENTS.iter().copied());
        emit_install(&app, "runtime", 0.25, 0.05, "Installing pinned model dependencies", None, None);
        let status = Command::new(&python)
            .args(pip_args)
            .status()
            .map_err(|error| error.to_string())?;
        if !status.success() {
            return Err("Could not install the pinned TripoSR Python dependencies.".into());
        }
        emit_install(&app, "runtime", 1.0, 0.23, "Private runtime ready", None, None);

        let client = Client::builder()
            .user_agent(format!("Still2Solid/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| error.to_string())?;
        let source_root = staging.join("source");
        for (index, (relative, expected_blob)) in SOURCE_FILES.iter().enumerate() {
            let url = format!(
                "https://raw.githubusercontent.com/VAST-AI-Research/TripoSR/{TRIPOSR_SOURCE_REVISION}/{relative}"
            );
            let destination = source_root.join(relative);
            let start = 0.23 + index as f64 / SOURCE_FILES.len() as f64 * 0.12;
            let end = 0.23 + (index + 1) as f64 / SOURCE_FILES.len() as f64 * 0.12;
            download(
                &app,
                &client,
                &url,
                &destination,
                "source",
                start,
                end,
                "Downloading pinned TripoSR source",
            )?;
            let actual = git_blob_sha1(&destination)?;
            if &actual != expected_blob {
                return Err(format!("Pinned TripoSR source verification failed for {relative}."));
            }
        }
        emit_install(&app, "source", 1.0, 0.35, "Pinned TripoSR source verified", None, None);

        let model_url = format!(
            "https://huggingface.co/stabilityai/TripoSR/resolve/{TRIPOSR_WEIGHT_REVISION}/model.ckpt?download=true"
        );
        let model = staging.join("model.ckpt");
        download(
            &app,
            &client,
            &model_url,
            &model,
            "weights",
            0.35,
            0.79,
            "Downloading the pinned TripoSR checkpoint",
        )?;
        if sha256_file(&model)? != TRIPOSR_WEIGHT_SHA256 {
            return Err("TripoSR checkpoint SHA-256 verification failed.".into());
        }
        emit_install(&app, "weights", 1.0, 0.79, "TripoSR checkpoint verified", None, None);

        let u2net = staging.join("u2net.onnx");
        download(
            &app,
            &client,
            "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx",
            &u2net,
            "foreground",
            0.79,
            0.91,
            "Downloading foreground-isolation support",
        )?;
        if md5_file(&u2net)? != U2NET_MD5 {
            return Err("Foreground-isolation asset checksum verification failed.".into());
        }

        fs::write(staging.join("worker.py"), WORKER).map_err(|error| error.to_string())?;
        fs::write(staging.join("config.yaml"), TRIPOSR_CONFIG).map_err(|error| error.to_string())?;
        fs::write(staging.join("dino-config.json"), DINO_CONFIG).map_err(|error| error.to_string())?;
        let manifest = serde_json::json!({
            "schema": INSTALL_SCHEMA,
            "modelId": "triposr",
            "sourceRevision": TRIPOSR_SOURCE_REVISION,
            "weightRevision": TRIPOSR_WEIGHT_REVISION,
            "weightSha256": TRIPOSR_WEIGHT_SHA256,
            "pythonVersion": python_version,
            "sourceManifest": "verified-upstream-layout-v2"
        });
        fs::write(
            staging.join("install.json"),
            serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;

        if final_root.exists() {
            fs::remove_dir_all(&final_root).map_err(|error| error.to_string())?;
        }
        fs::rename(&staging, &final_root).map_err(|error| error.to_string())?;
        emit_install(&app, "complete", 1.0, 1.0, "TripoSR is ready", None, None);
        Ok(())
    })();

    if let Err(error) = result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    get_model_runtime_states(app)?
        .into_iter()
        .next()
        .ok_or_else(|| "TripoSR installed, but its runtime state could not be read.".to_string())
}

#[tauri::command]
pub async fn install_model(
    app: AppHandle,
    state: State<'_, Arc<TripoInstallerState>>,
    model_id: String,
) -> Result<ModelRuntimeState, String> {
    if model_id != "triposr" {
        return Err("This installer manages the audited TripoSR adapter only.".to_string());
    }
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = state
            .lock
            .lock()
            .map_err(|_| "TripoSR installer lock is poisoned.".to_string())?;
        install_blocking(app)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::SOURCE_FILES;

    #[test]
    fn source_manifest_uses_real_upstream_transformer_layout() {
        assert!(SOURCE_FILES
            .iter()
            .any(|(path, _)| *path == "tsr/models/transformer/transformer_1d.py"));
        assert!(!SOURCE_FILES
            .iter()
            .any(|(path, _)| *path == "tsr/models/transformer.py"));
        assert!(!SOURCE_FILES
            .iter()
            .any(|(path, _)| *path == "tsr/models/renderer.py"));
    }
}
