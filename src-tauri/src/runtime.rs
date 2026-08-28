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
use uuid::Uuid;

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

const PYTHON_PACKAGES: &[&str] = &[
    "torch==2.13.0",
    "omegaconf==2.3.0",
    "Pillow==12.1.0",
    "einops==0.8.1",
    "transformers==4.35.0",
    "trimesh==4.0.5",
    "rembg[cpu]==2.0.77",
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
struct InstallProgress {
    model_id: String,
    stage: String,
    stage_progress: f64,
    overall_progress: f64,
    message: String,
    bytes_downloaded: Option<u64>,
    bytes_total: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallManifest {
    schema: u32,
    model_id: String,
    source_revision: String,
    weight_revision: String,
    weight_sha256: String,
    u2net_md5: String,
    python_version: String,
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

fn directory_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    if path.is_file() {
        return path.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    }
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| directory_size(&entry.path()))
        .sum()
}

fn read_install_manifest(root: &Path) -> Option<InstallManifest> {
    let content = fs::read_to_string(root.join("install.json")).ok()?;
    serde_json::from_str(&content).ok()
}

fn runtime_state_for(app: &AppHandle) -> Result<ModelRuntimeState, String> {
    let root = model_root(app)?;
    if !root.exists() {
        return Ok(ModelRuntimeState {
            model_id: "triposr".to_string(),
            status: "not-installed".to_string(),
            installed: false,
            verified: false,
            runtime_ready: false,
            can_generate: false,
            detail: "TripoSR production runtime is not installed.".to_string(),
            installed_bytes: 0,
            source_revision: TRIPOSR_SOURCE_REVISION.to_string(),
            weight_sha256: TRIPOSR_WEIGHT_SHA256.to_string(),
            python_version: None,
        });
    }

    let manifest = read_install_manifest(&root);
    let python = runtime_python(&root);
    let required_files_exist = python.exists()
        && root.join("model.ckpt").exists()
        && root.join("config.yaml").exists()
        && root.join("dino-config.json").exists()
        && root.join("rembg").join("u2net.onnx").exists()
        && root.join("worker.py").exists()
        && root.join("source").join("tsr").join("system.py").exists();

    let verified = manifest.as_ref().is_some_and(|manifest| {
        manifest.schema == INSTALL_SCHEMA
            && manifest.model_id == "triposr"
            && manifest.source_revision == TRIPOSR_SOURCE_REVISION
            && manifest.weight_revision == TRIPOSR_WEIGHT_REVISION
            && manifest.weight_sha256 == TRIPOSR_WEIGHT_SHA256
            && manifest.u2net_md5 == U2NET_MD5
            && required_files_exist
    });

    let python_version = manifest.as_ref().map(|value| value.python_version.clone());
    let status = if verified { "ready" } else { "broken" };
    let detail = if verified {
        "Pinned TripoSR source, weights, background-removal asset and isolated Python runtime are installed.".to_string()
    } else {
        "The TripoSR installation is incomplete or does not match the pinned M3 manifest. Reinstall it.".to_string()
    };

    Ok(ModelRuntimeState {
        model_id: "triposr".to_string(),
        status: status.to_string(),
        installed: true,
        verified,
        runtime_ready: verified,
        can_generate: verified,
        detail,
        installed_bytes: directory_size(&root),
        source_revision: TRIPOSR_SOURCE_REVISION.to_string(),
        weight_sha256: TRIPOSR_WEIGHT_SHA256.to_string(),
        python_version,
    })
}

#[tauri::command]
pub fn get_model_runtime_states(app: AppHandle) -> Result<Vec<ModelRuntimeState>, String> {
    Ok(vec![runtime_state_for(&app)?])
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
        InstallProgress {
            model_id: "triposr".to_string(),
            stage: stage.to_string(),
            stage_progress,
            overall_progress,
            message: message.into(),
            bytes_downloaded,
            bytes_total,
        },
    );
}

fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent("Still2Solid/0.3.0")
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|error| error.to_string())
}

fn download_file(
    client: &Client,
    app: &AppHandle,
    url: &str,
    destination: &Path,
    stage: &str,
    overall_start: f64,
    overall_span: f64,
) -> Result<u64, String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut response = client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("Download failed for {url}: {error}"))?;
    let total = response.content_length();
    let temporary = destination.with_extension("part");
    let mut file = File::create(&temporary).map_err(|error| error.to_string())?;
    let mut buffer = vec![0_u8; 1024 * 1024];
    let mut downloaded = 0_u64;
    let mut last_report = 0_u64;

    loop {
        let count = response.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        file.write_all(&buffer[..count]).map_err(|error| error.to_string())?;
        downloaded += count as u64;
        if downloaded.saturating_sub(last_report) >= 4 * 1024 * 1024 || total == Some(downloaded) {
            last_report = downloaded;
            let stage_progress = total
                .filter(|total| *total > 0)
                .map(|total| downloaded as f64 / total as f64)
                .unwrap_or(0.0);
            emit_install(
                app,
                stage,
                stage_progress,
                overall_start + stage_progress * overall_span,
                format!("Downloading {}", destination.file_name().and_then(|name| name.to_str()).unwrap_or("asset")),
                Some(downloaded),
                total,
            );
        }
    }
    file.sync_all().map_err(|error| error.to_string())?;
    fs::rename(&temporary, destination).map_err(|error| error.to_string())?;
    Ok(downloaded)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 4 * 1024 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn md5_file(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = BufReader::new(file);
    let mut context = md5::Context::new();
    let mut buffer = vec![0_u8; 4 * 1024 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        context.consume(&buffer[..count]);
    }
    Ok(format!("{:x}", context.compute()))
}

fn git_blob_sha1(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(format!("blob {}\0", bytes.len()).as_bytes());
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn verify_source_blob(path: &Path, expected: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let actual = git_blob_sha1(&bytes);
    if actual != expected {
        return Err(format!(
            "Pinned TripoSR source verification failed for {}: expected {expected}, got {actual}",
            path.display()
        ));
    }
    Ok(())
}

fn parse_python_probe(output: &str) -> Option<(u32, u32, String)> {
    let mut fields = output.trim().split('|');
    let major = fields.next()?.parse().ok()?;
    let minor = fields.next()?.parse().ok()?;
    let version = fields.next()?.to_string();
    Some((major, minor, version))
}

fn find_python() -> Result<(String, String), String> {
    let mut candidates = Vec::<String>::new();
    if let Ok(explicit) = std::env::var("STILL2SOLID_PYTHON") {
        if !explicit.trim().is_empty() {
            candidates.push(explicit);
        }
    }
    for candidate in ["python3.12", "python3.11", "python3"] {
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

    Err("M3 currently requires a local Python 3.11 or 3.12 interpreter to create the isolated TripoSR runtime. A bundled end-user Python runtime is a later packaging milestone.".to_string())
}

fn command_error(prefix: &str, output: std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if !stderr.trim().is_empty() { stderr } else { stdout };
    let tail: String = detail.chars().rev().take(4000).collect::<String>().chars().rev().collect();
    format!("{prefix}: {}", tail.trim())
}

fn run_checked(command: &mut Command, prefix: &str) -> Result<(), String> {
    let output = command.output().map_err(|error| format!("{prefix}: {error}"))?;
    if !output.status.success() {
        return Err(command_error(prefix, output));
    }
    Ok(())
}

fn install_triposr_blocking(app: AppHandle, state: Arc<RuntimeState>) -> Result<ModelRuntimeState, String> {
    let _guard = state.install_lock.lock().map_err(|_| "Model installer lock is poisoned.".to_string())?;
    let root = model_root(&app)?;
    let staging = root.with_extension("installing");
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&staging).map_err(|error| error.to_string())?;

    let install_result = (|| -> Result<(), String> {
        emit_install(&app, "runtime", 0.0, 0.01, "Checking the isolated Python runtime prerequisite", None, None);
        let (system_python, python_version) = find_python()?;
        emit_install(&app, "runtime", 0.08, 0.03, format!("Using Python {python_version} to create an isolated runtime"), None, None);

        let mut venv = Command::new(&system_python);
        venv.args(["-m", "venv"]).arg(staging.join("runtime"));
        run_checked(&mut venv, "Could not create the isolated Python runtime")?;

        let python = runtime_python(&staging);
        let mut pip_upgrade = Command::new(&python);
        pip_upgrade.args(["-m", "pip", "install", "--disable-pip-version-check", "--no-input", "--upgrade", "pip"]);
        run_checked(&mut pip_upgrade, "Could not prepare pip in the isolated runtime")?;

        emit_install(&app, "runtime", 0.25, 0.07, "Installing exact-version TripoSR runtime dependencies", None, None);
        let mut pip = Command::new(&python);
        pip.args(["-m", "pip", "install", "--disable-pip-version-check", "--no-input"]);
        pip.args(PYTHON_PACKAGES);
        run_checked(&mut pip, "Could not install the pinned TripoSR runtime dependencies")?;
        emit_install(&app, "runtime", 1.0, 0.20, "Isolated Python runtime ready", None, None);

        fs::write(staging.join("worker.py"), WORKER).map_err(|error| error.to_string())?;
        fs::write(staging.join("config.yaml"), TRIPOSR_CONFIG).map_err(|error| error.to_string())?;
        fs::write(staging.join("dino-config.json"), DINO_CONFIG).map_err(|error| error.to_string())?;

        let http = client()?;
        let source_root = staging.join("source");
        for (index, (relative, expected_blob)) in SOURCE_FILES.iter().enumerate() {
            let url = format!(
                "https://raw.githubusercontent.com/VAST-AI-Research/TripoSR/{TRIPOSR_SOURCE_REVISION}/{relative}"
            );
            let destination = source_root.join(relative);
            let local_fraction = index as f64 / SOURCE_FILES.len() as f64;
            download_file(&http, &app, &url, &destination, "source", 0.20 + local_fraction * 0.08, 0.08 / SOURCE_FILES.len() as f64)?;
            verify_source_blob(&destination, expected_blob)?;
        }
        emit_install(&app, "source", 1.0, 0.28, "Pinned TripoSR source verified against Git blob hashes", None, None);

        let weight_url = format!(
            "https://huggingface.co/stabilityai/TripoSR/resolve/{TRIPOSR_WEIGHT_REVISION}/model.ckpt?download=true"
        );
        let weight_path = staging.join("model.ckpt");
        download_file(&http, &app, &weight_url, &weight_path, "weights", 0.28, 0.48)?;
        emit_install(&app, "verify", 0.1, 0.77, "Verifying the 1.68 GB TripoSR checkpoint with SHA-256", None, None);
        let actual_weight_sha = sha256_file(&weight_path)?;
        if actual_weight_sha != TRIPOSR_WEIGHT_SHA256 {
            return Err(format!("TripoSR checkpoint SHA-256 mismatch: expected {TRIPOSR_WEIGHT_SHA256}, got {actual_weight_sha}"));
        }
        emit_install(&app, "verify", 1.0, 0.80, "TripoSR checkpoint checksum verified", None, None);

        let rembg_dir = staging.join("rembg");
        fs::create_dir_all(&rembg_dir).map_err(|error| error.to_string())?;
        let u2net_path = rembg_dir.join("u2net.onnx");
        download_file(
            &http,
            &app,
            "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx",
            &u2net_path,
            "background-model",
            0.80,
            0.14,
        )?;
        let actual_md5 = md5_file(&u2net_path)?;
        if actual_md5 != U2NET_MD5 {
            return Err(format!("U2Net checksum mismatch: expected {U2NET_MD5}, got {actual_md5}"));
        }
        emit_install(&app, "verify", 1.0, 0.95, "Foreground-isolation model checksum verified", None, None);

        let manifest = InstallManifest {
            schema: INSTALL_SCHEMA,
            model_id: "triposr".to_string(),
            source_revision: TRIPOSR_SOURCE_REVISION.to_string(),
            weight_revision: TRIPOSR_WEIGHT_REVISION.to_string(),
            weight_sha256: TRIPOSR_WEIGHT_SHA256.to_string(),
            u2net_md5: U2NET_MD5.to_string(),
            python_version,
        };
        fs::write(
            staging.join("install.json"),
            serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;

        if root.exists() {
            fs::remove_dir_all(&root).map_err(|error| error.to_string())?;
        }
        fs::rename(&staging, &root).map_err(|error| error.to_string())?;
        emit_install(&app, "complete", 1.0, 1.0, "TripoSR production runtime installed and verified", None, None);
        Ok(())
    })();

    if let Err(error) = install_result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    runtime_state_for(&app)
}

#[tauri::command]
pub async fn install_model(
    app: AppHandle,
    state: State<'_, Arc<RuntimeState>>,
    model_id: String,
) -> Result<ModelRuntimeState, String> {
    if model_id != "triposr" {
        return Err("M3 implements installation only for the audited TripoSR adapter.".to_string());
    }
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || install_triposr_blocking(app, state))
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
        return Err("M3 implements uninstall only for TripoSR.".to_string());
    }
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = state.install_lock.lock().map_err(|_| "Model installer lock is poisoned.".to_string())?;
        if !state.children.lock().map_err(|_| "Worker state is poisoned.".to_string())?.is_empty() {
            return Err("Stop the active generation before uninstalling TripoSR.".to_string());
        }
        let root = model_root(&app)?;
        if root.exists() {
            fs::remove_dir_all(&root).map_err(|error| error.to_string())?;
        }
        runtime_state_for(&app)
    })
    .await
    .map_err(|error| error.to_string())?
}

fn safe_source_extension(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "png"
    } else if lower.ends_with(".webp") {
        "webp"
    } else if lower.ends_with(".heic") || lower.ends_with(".heif") {
        "heic"
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
        return Err("Invalid quality preset.".to_string());
    }
    if !matches!(request.backend.as_str(), "auto" | "metal" | "cuda" | "cpu") {
        return Err("Invalid runtime backend.".to_string());
    }
    if request.source_bytes.is_empty() || request.source_bytes.len() > 64 * 1024 * 1024 {
        return Err("Source image must be between 1 byte and 64 MB.".to_string());
    }

    let runtime = runtime_state_for(&app)?;
    if !runtime.can_generate {
        return Err(runtime.detail);
    }
    let root = model_root(&app)?;
    let jobs_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("jobs");
    fs::create_dir_all(&jobs_root).map_err(|error| error.to_string())?;
    let job_dir = jobs_root.join(&request.job_id);
    if job_dir.exists() {
        fs::remove_dir_all(&job_dir).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&job_dir).map_err(|error| error.to_string())?;
    let source_path = job_dir.join(format!("source.{}", safe_source_extension(&request.source_name)));
    let output_path = job_dir.join("mesh.glb");
    let result_path = job_dir.join("result.json");
    fs::write(&source_path, &request.source_bytes).map_err(|error| error.to_string())?;

    let mut command = Command::new(runtime_python(&root));
    command
        .arg(root.join("worker.py"))
        .arg("--source-root")
        .arg(root.join("source"))
        .arg("--model-root")
        .arg(&root)
        .arg("--input")
        .arg(&source_path)
        .arg("--output")
        .arg(&output_path)
        .arg("--result-json")
        .arg(&result_path)
        .arg("--quality")
        .arg(&request.quality)
        .arg("--backend")
        .arg(&request.backend)
        .env("PYTHONUNBUFFERED", "1")
        .env("PYTORCH_ENABLE_MPS_FALLBACK", "1")
        .env("HF_HUB_OFFLINE", "1")
        .env("TRANSFORMERS_OFFLINE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if request.background_removal {
        command.arg("--remove-background");
    }

    let mut child = command.spawn().map_err(|error| format!("Could not launch the isolated TripoSR worker: {error}"))?;
    let stdout = child.stdout.take().ok_or_else(|| "Could not capture TripoSR worker progress.".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Could not capture TripoSR worker errors.".to_string())?;
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
    let progress_reader = thread::spawn(move || {
        use std::io::BufRead;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(progress) = serde_json::from_str::<WorkerProgress>(&line) {
                let _ = progress_app.emit(&format!("triposr-progress-{progress_job}"), progress);
            }
        }
    });

    let stderr_text = Arc::new(Mutex::new(String::new()));
    let stderr_target = stderr_text.clone();
    let stderr_reader = thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut text = String::new();
        let _ = reader.read_to_string(&mut text);
        if let Ok(mut target) = stderr_target.lock() {
            *target = text;
        }
    });

    let started = std::time::Instant::now();
    let status = loop {
        let status = child
            .lock()
            .map_err(|_| "Worker process lock is poisoned.".to_string())?
            .try_wait()
            .map_err(|error| error.to_string())?;
        if let Some(status) = status {
            break status;
        }
        thread::sleep(Duration::from_millis(100));
    };

    state.children.lock().map_err(|_| "Worker state is poisoned.".to_string())?.remove(&request.job_id);
    let _ = progress_reader.join();
    let _ = stderr_reader.join();

    let was_cancelled = state.cancelled.lock().map_err(|_| "Worker state is poisoned.".to_string())?.remove(&request.job_id);
    if was_cancelled {
        let _ = fs::remove_dir_all(&job_dir);
        return Err("Generation cancelled.".to_string());
    }
    if !status.success() {
        let stderr = stderr_text.lock().map_err(|_| "Worker error buffer is poisoned.".to_string())?.clone();
        let _ = fs::remove_dir_all(&job_dir);
        let detail: String = stderr.chars().rev().take(5000).collect::<String>().chars().rev().collect();
        return Err(if detail.trim().is_empty() {
            "The isolated TripoSR worker exited unexpectedly.".to_string()
        } else {
            format!("TripoSR generation failed: {}", detail.trim())
        });
    }

    let worker_result: WorkerResult = serde_json::from_slice(
        &fs::read(&result_path).map_err(|error| format!("Worker result is missing: {error}"))?,
    )
    .map_err(|error| format!("Worker result is invalid: {error}"))?;
    let glb = fs::read(&output_path).map_err(|error| format!("Generated GLB is missing: {error}"))?;
    let response = TripoGenerationResponse {
        job_id: request.job_id,
        model_id: "triposr".to_string(),
        elapsed_seconds: started.elapsed().as_secs_f64(),
        triangles: worker_result.triangles,
        textured: worker_result.textured,
        asset_base64: BASE64.encode(glb),
        asset_mime: "model/gltf-binary".to_string(),
        asset_filename: "still2solid-triposr.glb".to_string(),
        backend: worker_result.backend,
        mc_resolution: worker_result.mc_resolution,
        texture_resolution: worker_result.texture_resolution,
        warning: worker_result.warning,
    };
    let _ = fs::remove_dir_all(&job_dir);
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
        state.cancelled.lock().map_err(|_| "Worker state is poisoned.".to_string())?.insert(job_id);
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
    use super::{git_blob_sha1, parse_python_probe};

    #[test]
    fn computes_git_blob_sha1() {
        assert_eq!(git_blob_sha1(b"hello\n"), "ce013625030ba8dba906f756967f9e9ca394464a");
    }

    #[test]
    fn parses_python_probe() {
        assert_eq!(
            parse_python_probe("3|12|3.12.9\n"),
            Some((3, 12, "3.12.9".to_string()))
        );
    }
}
