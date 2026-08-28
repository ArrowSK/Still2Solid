use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

const SOURCE_REVISION: &str = "ff21fc491b4dc5314bf6734c7c0dabd86b5f5bb2";
const MODEL_SHA256: &str = "a3416e1cf654e7d4f5e75f116cec2c3f0a14501a77d30c2f6068bbda178de388";
const INSTALLER: &str = include_str!("../../workers/sf3d/install.py");
const WORKER: &str = include_str!("../../workers/sf3d/worker.py");

#[derive(Default)]
pub struct Sf3dState {
    children: Mutex<HashMap<String, Arc<Mutex<Child>>>>,
    cancelled: Mutex<HashSet<String>>,
    install_lock: Mutex<()>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf3dRuntimeState {
    model_id: String,
    status: String,
    installed: bool,
    verified: bool,
    runtime_ready: bool,
    can_generate: bool,
    detail: String,
    installed_bytes: u64,
    source_revision: String,
    weight_sha256: String,
    python_version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf3dGenerationRequest {
    job_id: String,
    quality: String,
    source_name: String,
    source_bytes: Vec<u8>,
    backend: String,
    background_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkerProgress {
    stage_id: String,
    stage_name: String,
    stage_progress: f64,
    overall_progress: f64,
    progress_is_estimated: bool,
    elapsed_seconds: f64,
    eta_seconds: f64,
    eta_confidence: String,
    status_message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkerResult {
    triangles: u64,
    textured: bool,
    backend: String,
    mc_resolution: u32,
    texture_resolution: u32,
    warning: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf3dGenerationResponse {
    job_id: String,
    model_id: String,
    elapsed_seconds: f64,
    triangles: u64,
    textured: bool,
    asset_base64: String,
    asset_mime: String,
    asset_filename: String,
    backend: String,
    mc_resolution: u32,
    texture_resolution: u32,
    warning: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallerProgress {
    stage: String,
    stage_progress: f64,
    overall_progress: f64,
    message: String,
}

#[derive(Debug, Serialize)]
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

fn root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("models")
        .join("sf3d"))
}

fn venv_python(root: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        root.join("runtime").join("Scripts").join("python.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        root.join("runtime").join("bin").join("python")
    }
}

fn bundled_python_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let Ok(resources) = app.path().resource_dir() else {
        return vec![];
    };
    #[cfg(target_os = "windows")]
    let executable = Path::new("python.exe");
    #[cfg(not(target_os = "windows"))]
    let executable = Path::new("bin/python3");

    vec![
        resources.join("python").join(executable),
        resources.join("resources").join("python").join(executable),
    ]
}

fn probe_python(candidate: &Path) -> Option<String> {
    let output = Command::new(candidate)
        .args([
            "-c",
            "import sys; print(f'{sys.version_info.major}|{sys.version_info.minor}|{sys.version.split()[0]}')",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut fields = text.trim().split('|');
    let major = fields.next()?.parse::<u32>().ok()?;
    let minor = fields.next()?.parse::<u32>().ok()?;
    let version = fields.next()?.to_string();
    (major == 3 && matches!(minor, 11 | 12)).then_some(version)
}

fn find_base_python(app: &AppHandle) -> Result<(PathBuf, String), String> {
    let mut candidates = bundled_python_candidates(app);
    if let Ok(explicit) = std::env::var("STILL2SOLID_PYTHON") {
        if !explicit.trim().is_empty() {
            candidates.push(PathBuf::from(explicit));
        }
    }
    #[cfg(target_os = "windows")]
    for candidate in ["python.exe", "python3.12.exe", "python3.11.exe"] {
        candidates.push(PathBuf::from(candidate));
    }
    #[cfg(not(target_os = "windows"))]
    for candidate in ["python3.12", "python3.11", "python3"] {
        candidates.push(PathBuf::from(candidate));
    }

    for candidate in candidates {
        if let Some(version) = probe_python(&candidate) {
            return Ok((candidate, version));
        }
    }
    Err("Stable Fast 3D needs Still2Solid's bundled Python 3.12 runtime. Source builds may provide Python 3.11/3.12 with STILL2SOLID_PYTHON.".to_string())
}

fn directory_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    if path.is_file() {
        return path.metadata().map(|value| value.len()).unwrap_or(0);
    }
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| directory_size(&entry.path()))
        .sum()
}

fn state_for(app: &AppHandle) -> Result<Sf3dRuntimeState, String> {
    let root = root(app)?;
    if !root.exists() {
        return Ok(Sf3dRuntimeState {
            model_id: "sf3d".into(),
            status: "not-installed".into(),
            installed: false,
            verified: false,
            runtime_ready: false,
            can_generate: false,
            detail: "Stable Fast 3D is optional and gated. Review its licence and hardware requirements before installing it.".into(),
            installed_bytes: 0,
            source_revision: SOURCE_REVISION.into(),
            weight_sha256: MODEL_SHA256.into(),
            python_version: None,
        });
    }

    let manifest_path = root.join("install.json");
    let manifest = fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    let required = venv_python(&root).exists()
        && root.join("worker.py").exists()
        && root.join("source").join("sf3d").join("system.py").exists()
        && root.join("model").join("config.yaml").exists()
        && root.join("model").join("model.safetensors").exists()
        && root.join("rembg").join("u2net.onnx").exists();
    let verified = manifest.as_ref().is_some_and(|value| {
        value.get("modelId").and_then(|v| v.as_str()) == Some("sf3d")
            && value.get("sourceRevision").and_then(|v| v.as_str()) == Some(SOURCE_REVISION)
            && value.get("weightSha256").and_then(|v| v.as_str()) == Some(MODEL_SHA256)
            && value.get("licenseAccepted").and_then(|v| v.as_bool()) == Some(true)
            && value.get("tokenStored").and_then(|v| v.as_bool()) == Some(false)
            && required
    });
    let python_version = manifest
        .as_ref()
        .and_then(|value| value.get("pythonVersion"))
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);

    Ok(Sf3dRuntimeState {
        model_id: "sf3d".into(),
        status: if verified { "ready" } else { "broken" }.into(),
        installed: true,
        verified,
        runtime_ready: verified,
        can_generate: verified,
        detail: if verified {
            "Stable Fast 3D is installed locally. The gated access token was used only for installation and was not stored.".into()
        } else {
            "The Stable Fast 3D installation is incomplete or no longer matches the audited M8 manifest. Reinstall it.".into()
        },
        installed_bytes: directory_size(&root),
        source_revision: SOURCE_REVISION.into(),
        weight_sha256: MODEL_SHA256.into(),
        python_version,
    })
}

#[tauri::command]
pub fn get_sf3d_runtime_state(app: AppHandle) -> Result<Sf3dRuntimeState, String> {
    state_for(&app)
}

fn emit_install(app: &AppHandle, progress: InstallerProgress) {
    let _ = app.emit(
        "model-install-progress",
        InstallProgressEvent {
            model_id: "sf3d".into(),
            stage: progress.stage,
            stage_progress: progress.stage_progress,
            overall_progress: progress.overall_progress,
            message: progress.message,
            bytes_downloaded: None,
            bytes_total: None,
        },
    );
}

fn install_blocking(
    app: AppHandle,
    state: Arc<Sf3dState>,
    hf_token: String,
    accepted_license: bool,
) -> Result<Sf3dRuntimeState, String> {
    if !accepted_license {
        return Err("Review and accept the Stability AI Community License before installing Stable Fast 3D.".into());
    }
    let token = hf_token.trim().to_string();
    if token.len() < 8 {
        return Err("Enter a Hugging Face read token that has access to the gated Stable Fast 3D repository.".into());
    }
    let _guard = state
        .install_lock
        .lock()
        .map_err(|_| "Stable Fast 3D installer lock is poisoned.".to_string())?;
    let final_root = root(&app)?;
    let staging = final_root.with_extension("installing");
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
    let installer_path = staging.join("installer.py");
    fs::write(&installer_path, INSTALLER).map_err(|error| error.to_string())?;

    let install_result = (|| -> Result<(), String> {
        let (base_python, version) = find_base_python(&app)?;
        let _ = app.emit(
            "model-install-progress",
            InstallProgressEvent {
                model_id: "sf3d".into(),
                stage: "runtime".into(),
                stage_progress: 0.0,
                overall_progress: 0.01,
                message: format!("Using Still2Solid Python {version} to create the private model runtime"),
                bytes_downloaded: None,
                bytes_total: None,
            },
        );

        let mut command = Command::new(base_python);
        command
            .arg(&installer_path)
            .arg("--root")
            .arg(&staging)
            .env("STILL2SOLID_HF_TOKEN", &token)
            .env("HF_HUB_DISABLE_TELEMETRY", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|error| format!("Could not start the Stable Fast 3D installer: {error}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Could not read Stable Fast 3D installer progress.".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Could not read Stable Fast 3D installer errors.".to_string())?;
        let progress_app = app.clone();
        let progress_thread = thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Ok(progress) = serde_json::from_str::<InstallerProgress>(&line) {
                    emit_install(&progress_app, progress);
                }
            }
        });
        let stderr_thread = thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut text = String::new();
            let _ = reader.read_to_string(&mut text);
            text
        });
        let status = child.wait().map_err(|error| error.to_string())?;
        let _ = progress_thread.join();
        let stderr = stderr_thread.join().unwrap_or_default();
        if !status.success() {
            let tail: String = stderr
                .chars()
                .rev()
                .take(6000)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            return Err(if tail.trim().is_empty() {
                "Stable Fast 3D installation failed. Check model access and the model's native build prerequisites.".into()
            } else {
                format!("Stable Fast 3D installation failed: {}", tail.trim())
            });
        }
        fs::write(staging.join("worker.py"), WORKER).map_err(|error| error.to_string())?;
        let staged = state_for_staging(&staging)?;
        if !staged {
            return Err("Stable Fast 3D installer completed but the audited runtime manifest did not verify.".into());
        }
        let _ = fs::remove_file(&installer_path);
        if final_root.exists() {
            fs::remove_dir_all(&final_root).map_err(|error| error.to_string())?;
        }
        fs::rename(&staging, &final_root).map_err(|error| error.to_string())?;
        Ok(())
    })();

    if let Err(error) = install_result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    state_for(&app)
}

fn state_for_staging(root: &Path) -> Result<bool, String> {
    let value: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("install.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(value.get("modelId").and_then(|v| v.as_str()) == Some("sf3d")
        && value.get("sourceRevision").and_then(|v| v.as_str()) == Some(SOURCE_REVISION)
        && value.get("weightSha256").and_then(|v| v.as_str()) == Some(MODEL_SHA256)
        && value.get("tokenStored").and_then(|v| v.as_bool()) == Some(false)
        && venv_python(root).exists()
        && root.join("worker.py").exists()
        && root.join("source/sf3d/system.py").exists()
        && root.join("model/config.yaml").exists()
        && root.join("model/model.safetensors").exists())
}

#[tauri::command]
pub async fn install_sf3d(
    app: AppHandle,
    state: State<'_, Arc<Sf3dState>>,
    hf_token: String,
    accepted_license: bool,
) -> Result<Sf3dRuntimeState, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || install_blocking(app, state, hf_token, accepted_license))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn uninstall_sf3d(
    app: AppHandle,
    state: State<'_, Arc<Sf3dState>>,
) -> Result<Sf3dRuntimeState, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = state
            .install_lock
            .lock()
            .map_err(|_| "Stable Fast 3D installer lock is poisoned.".to_string())?;
        if !state
            .children
            .lock()
            .map_err(|_| "Stable Fast 3D worker state is poisoned.".to_string())?
            .is_empty()
        {
            return Err("Stop the active Stable Fast 3D generation before uninstalling it.".into());
        }
        let root = root(&app)?;
        if root.exists() {
            fs::remove_dir_all(root).map_err(|error| error.to_string())?;
        }
        state_for(&app)
    })
    .await
    .map_err(|error| error.to_string())?
}

fn source_extension(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "png"
    } else if lower.ends_with(".webp") {
        "webp"
    } else {
        "jpg"
    }
}

fn generate_blocking(
    app: AppHandle,
    state: Arc<Sf3dState>,
    request: Sf3dGenerationRequest,
) -> Result<Sf3dGenerationResponse, String> {
    if !matches!(request.quality.as_str(), "fast" | "standard" | "best") {
        return Err("Invalid quality preset.".into());
    }
    if !matches!(request.backend.as_str(), "auto" | "metal" | "cuda" | "cpu") {
        return Err("Invalid runtime backend.".into());
    }
    if request.source_bytes.is_empty() || request.source_bytes.len() > 64 * 1024 * 1024 {
        return Err("Source image must be between 1 byte and 64 MB.".into());
    }
    let runtime = state_for(&app)?;
    if !runtime.can_generate {
        return Err(runtime.detail);
    }
    let root = root(&app)?;
    let jobs = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("jobs");
    fs::create_dir_all(&jobs).map_err(|error| error.to_string())?;
    let job = jobs.join(format!("sf3d-{}", request.job_id));
    if job.exists() {
        fs::remove_dir_all(&job).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&job).map_err(|error| error.to_string())?;
    let source = job.join(format!("source.{}", source_extension(&request.source_name)));
    let output = job.join("mesh.glb");
    let result_json = job.join("result.json");
    fs::write(&source, &request.source_bytes).map_err(|error| error.to_string())?;

    let mut command = Command::new(venv_python(&root));
    command
        .arg(root.join("worker.py"))
        .arg("--source-root")
        .arg(root.join("source"))
        .arg("--model-root")
        .arg(&root)
        .arg("--input")
        .arg(&source)
        .arg("--output")
        .arg(&output)
        .arg("--result-json")
        .arg(&result_json)
        .arg("--quality")
        .arg(&request.quality)
        .arg("--backend")
        .arg(&request.backend)
        .env("PYTHONUNBUFFERED", "1")
        .env("HF_HUB_OFFLINE", "1")
        .env("TRANSFORMERS_OFFLINE", "1")
        .env("HF_HUB_DISABLE_TELEMETRY", "1")
        .env("PYTORCH_ENABLE_MPS_FALLBACK", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if request.background_removal {
        command.arg("--remove-background");
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not launch the isolated Stable Fast 3D worker: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Could not capture Stable Fast 3D progress.".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Could not capture Stable Fast 3D errors.".to_string())?;
    let child = Arc::new(Mutex::new(child));
    state
        .children
        .lock()
        .map_err(|_| "Stable Fast 3D worker state is poisoned.".to_string())?
        .insert(request.job_id.clone(), child.clone());
    state
        .cancelled
        .lock()
        .map_err(|_| "Stable Fast 3D worker state is poisoned.".to_string())?
        .remove(&request.job_id);

    let progress_app = app.clone();
    let progress_job = request.job_id.clone();
    let progress_thread = thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(progress) = serde_json::from_str::<WorkerProgress>(&line) {
                let _ = progress_app.emit(&format!("sf3d-progress-{progress_job}"), progress);
            }
        }
    });
    let stderr_thread = thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut text = String::new();
        let _ = reader.read_to_string(&mut text);
        text
    });

    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child
            .lock()
            .map_err(|_| "Stable Fast 3D process lock is poisoned.".to_string())?
            .try_wait()
            .map_err(|error| error.to_string())?
        {
            break status;
        }
        thread::sleep(Duration::from_millis(100));
    };
    state
        .children
        .lock()
        .map_err(|_| "Stable Fast 3D worker state is poisoned.".to_string())?
        .remove(&request.job_id);
    let _ = progress_thread.join();
    let stderr = stderr_thread.join().unwrap_or_default();
    let cancelled = state
        .cancelled
        .lock()
        .map_err(|_| "Stable Fast 3D worker state is poisoned.".to_string())?
        .remove(&request.job_id);
    if cancelled {
        let _ = fs::remove_dir_all(&job);
        return Err("Generation cancelled.".into());
    }
    if !status.success() {
        let tail: String = stderr
            .chars()
            .rev()
            .take(6000)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        let _ = fs::remove_dir_all(&job);
        return Err(if tail.trim().is_empty() {
            "The isolated Stable Fast 3D worker exited unexpectedly.".into()
        } else {
            format!("Stable Fast 3D generation failed: {}", tail.trim())
        });
    }

    let worker: WorkerResult = serde_json::from_slice(
        &fs::read(&result_json).map_err(|error| format!("Worker result is missing: {error}"))?,
    )
    .map_err(|error| format!("Worker result is invalid: {error}"))?;
    let glb = fs::read(&output).map_err(|error| format!("Generated GLB is missing: {error}"))?;
    let response = Sf3dGenerationResponse {
        job_id: request.job_id,
        model_id: "sf3d".into(),
        elapsed_seconds: started.elapsed().as_secs_f64(),
        triangles: worker.triangles,
        textured: worker.textured,
        asset_base64: BASE64.encode(glb),
        asset_mime: "model/gltf-binary".into(),
        asset_filename: "still2solid-sf3d.glb".into(),
        backend: worker.backend,
        mc_resolution: worker.mc_resolution,
        texture_resolution: worker.texture_resolution,
        warning: worker.warning,
    };
    let _ = fs::remove_dir_all(&job);
    Ok(response)
}

#[tauri::command]
pub async fn generate_sf3d(
    app: AppHandle,
    state: State<'_, Arc<Sf3dState>>,
    request: Sf3dGenerationRequest,
) -> Result<Sf3dGenerationResponse, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || generate_blocking(app, state, request))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn cancel_sf3d(
    state: State<'_, Arc<Sf3dState>>,
    job_id: String,
) -> Result<bool, String> {
    let child = state
        .children
        .lock()
        .map_err(|_| "Stable Fast 3D worker state is poisoned.".to_string())?
        .get(&job_id)
        .cloned();
    if let Some(child) = child {
        state
            .cancelled
            .lock()
            .map_err(|_| "Stable Fast 3D worker state is poisoned.".to_string())?
            .insert(job_id);
        child
            .lock()
            .map_err(|_| "Stable Fast 3D process lock is poisoned.".to_string())?
            .kill()
            .map_err(|error| error.to_string())?;
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{source_extension, SOURCE_REVISION};

    #[test]
    fn keeps_source_extension_conservative() {
        assert_eq!(source_extension("photo.png"), "png");
        assert_eq!(source_extension("photo.webp"), "webp");
        assert_eq!(source_extension("photo.heic"), "jpg");
    }

    #[test]
    fn source_revision_is_immutable_commit() {
        assert_eq!(SOURCE_REVISION.len(), 40);
        assert!(SOURCE_REVISION.chars().all(|value| value.is_ascii_hexdigit()));
    }
}
