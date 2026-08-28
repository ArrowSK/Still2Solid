use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

const TRIPOSR_SOURCE_REVISION: &str = "107cefdc244c39106fa830359024f6a2f1c78871";
const TRIPOSR_WEIGHT_REVISION: &str = "5b521936b01fbe1890f6f9baed0254ab6351c04a";
const TRIPOSR_WEIGHT_SHA256: &str = "429e2c6b22a0923967459de24d67f05962b235f79cde6b032aa7ed2ffcd970ee";
const U2NET_MD5: &str = "60024c5c889badc19c04ad937298a77b";
const INSTALL_SCHEMA: u32 = 1;

const WORKER: &str = include_str!("../../workers/triposr/worker.py");
const TRIPOSR_CONFIG: &str = include_str!("../../workers/triposr/config.yaml");
const DINO_CONFIG: &str = include_str!("../../workers/triposr/dino-config.json");

const SOURCE_FILES: &[(&str, &str)] = &[
    ("LICENSE", "6c440ca1ef32cedcc2257b99953add129199ed26"),
    ("tsr/system.py", "bcdb69b2e5c85b3c0ffebd7ff82fc4fe7b3d543e"),
    ("tsr/utils.py", "9758f2e44cd4091f2124929edc89b59e98fff667"),
    ("tsr/models/transformer.py", "a0659f42dbd3468382f19902095896d0fbf1d7b0"),
    ("tsr/models/renderer.py", "ad6235c9de295fa4c0df9f4d68b16fe4b6591628"),
    ("tsr/models/isosurface.py", "a3a4a6f4da8f085e5b403ab434c6d76b02eedb57"),
    ("tsr/models/tokenizers/image.py", "634965db5214f25579ea1636224a5682086a3142"),
    ("tsr/models/tokenizers/triplane.py", "28ea69b5c80f91783052a3a9419e649094609d96"),
    ("tsr/models/nerf_renderer.py", "c2158e96583d506d9ccfe08cac17fb93d672e6ec"),
    ("tsr/models/network_utils.py", "a498251767712e27938532941638002d400248cd"),
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
pub struct RuntimeState {
    children: Mutex<HashMap<String, Arc<Mutex<Child>>>>,
    cancelled: Mutex<HashSet<String>>,
    install_lock: Mutex<()>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimeState {
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TripoGenerationRequest {
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
pub struct TripoGenerationResponse {
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

fn parse_python_probe(text: &str) -> Option<(u32, u32, String)> {
    let mut parts = text.trim().split('|');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let version = parts.next()?.to_string();
    Some((major, minor, version))
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
        if !explicit.trim().is_empty() && !candidates.iter().any(|value| value == &explicit) {
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
        if let Some((3, minor, version)) = parse_python_probe(&text) {
            if minor == 11 || minor == 12 {
                return Ok((candidate, version));
            }
        }
    }

    Err("Still2Solid could not find its bundled Python 3.12 runtime. Development builds may use Python 3.11/3.12 via STILL2SOLID_PYTHON; release builds should be repaired or reinstalled instead of asking the user to configure Python manually.".to_string())
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

fn git_blob_sha1(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha1::new();
    hasher.update(format!("blob {}\0", bytes.len()).as_bytes());
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn md5_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", md5::compute(bytes)))
}

fn manifest_verified(root: &Path) -> bool {
    let manifest = fs::read_to_string(root.join("install.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    manifest.as_ref().is_some_and(|value| {
        value.get("schema").and_then(|v| v.as_u64()) == Some(INSTALL_SCHEMA as u64)
            && value.get("modelId").and_then(|v| v.as_str()) == Some("triposr")
            && value.get("sourceRevision").and_then(|v| v.as_str()) == Some(TRIPOSR_SOURCE_REVISION)
            && value.get("weightRevision").and_then(|v| v.as_str()) == Some(TRIPOSR_WEIGHT_REVISION)
            && value.get("weightSha256").and_then(|v| v.as_str()) == Some(TRIPOSR_WEIGHT_SHA256)
            && runtime_python(root).exists()
            && root.join("worker.py").exists()
            && root.join("model.ckpt").exists()
            && root.join("u2net.onnx").exists()
            && root.join("source/tsr/system.py").exists()
    })
}

fn state_for(app: &AppHandle) -> Result<ModelRuntimeState, String> {
    let root = model_root(app)?;
    if !root.exists() {
        return Ok(ModelRuntimeState {
            model_id: "triposr".into(),
            status: "not-installed".into(),
            installed: false,
            verified: false,
            runtime_ready: false,
            can_generate: false,
            detail: "TripoSR is not installed yet. Model Manager can install the pinned, checksum-verified local runtime.".into(),
            installed_bytes: 0,
            source_revision: TRIPOSR_SOURCE_REVISION.into(),
            weight_sha256: TRIPOSR_WEIGHT_SHA256.into(),
            python_version: None,
        });
    }
    let verified = manifest_verified(&root);
    let manifest = fs::read_to_string(root.join("install.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    let python_version = manifest
        .as_ref()
        .and_then(|value| value.get("pythonVersion"))
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);
    Ok(ModelRuntimeState {
        model_id: "triposr".into(),
        status: if verified { "ready" } else { "broken" }.into(),
        installed: true,
        verified,
        runtime_ready: verified,
        can_generate: verified,
        detail: if verified {
            "TripoSR is installed locally and matches the pinned M3 manifest.".into()
        } else {
            "The TripoSR installation is incomplete or no longer matches the audited manifest. Reinstall it from Model Manager.".into()
        },
        installed_bytes: directory_size(&root),
        source_revision: TRIPOSR_SOURCE_REVISION.into(),
        weight_sha256: TRIPOSR_WEIGHT_SHA256.into(),
        python_version,
    })
}

#[tauri::command]
pub fn get_model_runtime_states(app: AppHandle) -> Result<Vec<ModelRuntimeState>, String> {
    Ok(vec![state_for(&app)?])
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
            .filter(|total| *total > 0)
            .map(|total| (downloaded as f64 / total as f64).clamp(0.0, 1.0))
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

fn install_blocking(app: AppHandle, state: Arc<RuntimeState>) -> Result<ModelRuntimeState, String> {
    let _guard = state
        .install_lock
        .lock()
        .map_err(|_| "Installer lock is poisoned.".to_string())?;
    let final_root = model_root(&app)?;
    let staging = final_root.with_extension("installing");
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&staging).map_err(|error| error.to_string())?;

    let install_result = (|| -> Result<(), String> {
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
        let mut pip_args = vec!["-m", "pip", "install", "--disable-pip-version-check", "--no-input"];
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
            .user_agent("Still2Solid/0.8")
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
            download(&app, &client, &url, &destination, "source", start, end, "Downloading pinned TripoSR source")?;
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
        download(&app, &client, &model_url, &model, "weights", 0.35, 0.79, "Downloading the pinned TripoSR checkpoint")?;
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
        emit_install(&app, "foreground", 1.0, 0.91, "Foreground-isolation support verified", None, None);

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
        });
        fs::write(
            staging.join("install.json"),
            serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        if !manifest_verified(&staging) {
            return Err("Installer completed but the staged TripoSR runtime did not verify.".into());
        }
        emit_install(&app, "verify", 1.0, 0.98, "Installation verified", None, None);

        if final_root.exists() {
            fs::remove_dir_all(&final_root).map_err(|error| error.to_string())?;
        }
        fs::rename(&staging, &final_root).map_err(|error| error.to_string())?;
        emit_install(&app, "complete", 1.0, 1.0, "TripoSR is ready", None, None);
        Ok(())
    })();

    if let Err(error) = install_result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    state_for(&app)
}

#[tauri::command]
pub async fn install_model(
    app: AppHandle,
    state: State<'_, Arc<RuntimeState>>,
    model_id: String,
) -> Result<ModelRuntimeState, String> {
    if model_id != "triposr" {
        return Err("Only the audited TripoSR M3 adapter can be installed by this command.".into());
    }
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || install_blocking(app, state))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn uninstall_model(
    app: AppHandle,
    state: State<'_, Arc<RuntimeState>>,
    model_id: String,
) -> Result<ModelRuntimeState, String> {
    if model_id != "triposr" {
        return Err("Only TripoSR is managed by the production runtime command.".into());
    }
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = state
            .install_lock
            .lock()
            .map_err(|_| "Installer lock is poisoned.".to_string())?;
        if !state
            .children
            .lock()
            .map_err(|_| "Worker state is poisoned.".to_string())?
            .is_empty()
        {
            return Err("Stop the active generation before uninstalling TripoSR.".into());
        }
        let root = model_root(&app)?;
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
    state: Arc<RuntimeState>,
    request: TripoGenerationRequest,
) -> Result<TripoGenerationResponse, String> {
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
    let root = model_root(&app)?;
    let jobs = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("jobs");
    fs::create_dir_all(&jobs).map_err(|error| error.to_string())?;
    let job = jobs.join(format!("triposr-{}", request.job_id));
    if job.exists() {
        fs::remove_dir_all(&job).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&job).map_err(|error| error.to_string())?;
    let source = job.join(format!("source.{}", source_extension(&request.source_name)));
    let output = job.join("mesh.glb");
    let result_json = job.join("result.json");
    fs::write(&source, &request.source_bytes).map_err(|error| error.to_string())?;

    let mut command = Command::new(runtime_python(&root));
    command
        .arg(root.join("worker.py"))
        .arg("--root")
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
        .env("U2NET_HOME", &root)
        .env("PYTORCH_ENABLE_MPS_FALLBACK", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if request.background_removal {
        command.arg("--remove-background");
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not launch the isolated TripoSR worker: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Could not capture worker progress.".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Could not capture worker errors.".to_string())?;
    let child = Arc::new(Mutex::new(child));
    state
        .children
        .lock()
        .map_err(|_| "Worker state is poisoned.".to_string())?
        .insert(request.job_id.clone(), child.clone());
    state
        .cancelled
        .lock()
        .map_err(|_| "Worker state is poisoned.".to_string())?
        .remove(&request.job_id);

    let progress_app = app.clone();
    let progress_job = request.job_id.clone();
    let progress_thread = thread::spawn(move || {
        for line in std::io::BufRead::lines(BufReader::new(stdout)).map_while(Result::ok) {
            if let Ok(progress) = serde_json::from_str::<WorkerProgress>(&line) {
                let _ = progress_app.emit(&format!("triposr-progress-{progress_job}"), progress);
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
            .map_err(|_| "Worker process lock is poisoned.".to_string())?
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
        .map_err(|_| "Worker state is poisoned.".to_string())?
        .remove(&request.job_id);
    let _ = progress_thread.join();
    let stderr = stderr_thread.join().unwrap_or_default();
    let cancelled = state
        .cancelled
        .lock()
        .map_err(|_| "Worker state is poisoned.".to_string())?
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
            "The isolated TripoSR worker exited unexpectedly.".into()
        } else {
            format!("TripoSR generation failed: {}", tail.trim())
        });
    }

    let worker: WorkerResult = serde_json::from_slice(
        &fs::read(&result_json).map_err(|error| format!("Worker result is missing: {error}"))?,
    )
    .map_err(|error| format!("Worker result is invalid: {error}"))?;
    let glb = fs::read(&output).map_err(|error| format!("Generated GLB is missing: {error}"))?;
    let response = TripoGenerationResponse {
        job_id: request.job_id,
        model_id: "triposr".into(),
        elapsed_seconds: started.elapsed().as_secs_f64(),
        triangles: worker.triangles,
        textured: worker.textured,
        asset_base64: BASE64.encode(glb),
        asset_mime: "model/gltf-binary".into(),
        asset_filename: "still2solid-triposr.glb".into(),
        backend: worker.backend,
        mc_resolution: worker.mc_resolution,
        texture_resolution: worker.texture_resolution,
        warning: worker.warning,
    };
    let _ = fs::remove_dir_all(&job);
    Ok(response)
}

#[tauri::command]
pub async fn generate_triposr(
    app: AppHandle,
    state: State<'_, Arc<RuntimeState>>,
    request: TripoGenerationRequest,
) -> Result<TripoGenerationResponse, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || generate_blocking(app, state, request))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn cancel_generation(
    state: State<'_, Arc<RuntimeState>>,
    job_id: String,
) -> Result<bool, String> {
    let child = state
        .children
        .lock()
        .map_err(|_| "Worker state is poisoned.".to_string())?
        .get(&job_id)
        .cloned();
    if let Some(child) = child {
        state
            .cancelled
            .lock()
            .map_err(|_| "Worker state is poisoned.".to_string())?
            .insert(job_id);
        child
            .lock()
            .map_err(|_| "Worker process lock is poisoned.".to_string())?
            .kill()
            .map_err(|error| error.to_string())?;
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{parse_python_probe, source_extension, TRIPOSR_SOURCE_REVISION, TRIPOSR_WEIGHT_SHA256};

    #[test]
    fn parses_supported_python_version() {
        let parsed = parse_python_probe("3|12|3.12.4\n").expect("probe parses");
        assert_eq!(parsed.0, 3);
        assert_eq!(parsed.1, 12);
        assert_eq!(parsed.2, "3.12.4");
    }

    #[test]
    fn source_extension_is_conservative() {
        assert_eq!(source_extension("thing.png"), "png");
        assert_eq!(source_extension("thing.webp"), "webp");
        assert_eq!(source_extension("thing.heic"), "jpg");
    }

    #[test]
    fn source_revision_is_immutable_commit() {
        assert_eq!(TRIPOSR_SOURCE_REVISION.len(), 40);
        assert!(TRIPOSR_SOURCE_REVISION.chars().all(|value| value.is_ascii_hexdigit()));
    }

    #[test]
    fn model_hash_is_sha256() {
        assert_eq!(TRIPOSR_WEIGHT_SHA256.len(), 64);
        assert!(TRIPOSR_WEIGHT_SHA256.chars().all(|value| value.is_ascii_hexdigit()));
    }
}