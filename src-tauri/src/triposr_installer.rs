use crate::runtime::{get_model_runtime_states, ModelRuntimeState};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Deserialize;
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::Sha256;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

const TRIPOSR_SOURCE_REVISION: &str = "107cefdc244c39106fa830359024f6a2f1c78871";
const TRIPOSR_WEIGHT_REVISION: &str = "5b521936b01fbe1890f6f9baed0254ab6351c04a";
const TRIPOSR_WEIGHT_SHA256: &str = "429e2c6b22a0923967459de24d67f05962b235f79cde6b032aa7ed2ffcd970ee";
const U2NET_MD5: &str = "60024c5c889badc19c04ad937298a77b";
const INSTALL_SCHEMA: u32 = 1;
const HTTP_ATTEMPTS: usize = 3;

const WORKER: &str = include_str!("../../workers/triposr/worker.py");
const TRIPOSR_CONFIG: &str = include_str!("../../workers/triposr/config.yaml");
const DINO_CONFIG: &str = include_str!("../../workers/triposr/dino-config.json");

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

#[derive(Debug, Deserialize)]
struct GitBlobResponse {
    sha: String,
    encoding: String,
    content: String,
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

fn retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(700 * (1u64 << attempt.saturating_sub(1).min(3)))
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

fn validate_https_url(value: &str) -> Result<String, String> {
    let parsed = reqwest::Url::parse(value.trim())
        .map_err(|_| "The alternate model link is not a valid URL.".to_string())?;
    if parsed.scheme() != "https" {
        return Err("Alternate model links must use HTTPS.".to_string());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("Alternate model links must not contain embedded credentials.".to_string());
    }
    Ok(parsed.to_string())
}

fn download_http_once(
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
    if temporary.exists() {
        let _ = fs::remove_file(&temporary);
    }

    let mut response = client
        .get(url)
        .send()
        .map_err(|error| format!("Network error while downloading {url}: {error}"))?;
    if response.url().scheme() != "https" {
        return Err("Still2Solid refused a download redirected away from HTTPS.".to_string());
    }
    if !response.status().is_success() {
        let status = response.status();
        return Err(format!("HTTP {status} while downloading {url}"));
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
    if destination.exists() {
        fs::remove_file(destination).map_err(|error| error.to_string())?;
    }
    fs::rename(&temporary, destination).map_err(|error| error.to_string())?;
    Ok(())
}

fn download_http_with_retries(
    app: &AppHandle,
    client: &Client,
    urls: &[String],
    destination: &Path,
    stage: &str,
    overall_start: f64,
    overall_end: f64,
    message: &str,
) -> Result<(), String> {
    let mut failures = Vec::new();
    for (source_index, url) in urls.iter().enumerate() {
        for attempt in 1..=HTTP_ATTEMPTS {
            let attempt_message = if source_index == 0 {
                format!("{message} · attempt {attempt}/{HTTP_ATTEMPTS}")
            } else {
                format!("{message} · fallback source {} · attempt {attempt}/{HTTP_ATTEMPTS}", source_index + 1)
            };
            emit_install(app, stage, 0.0, overall_start, attempt_message, None, None);
            match download_http_once(
                app,
                client,
                url,
                destination,
                stage,
                overall_start,
                overall_end,
                message,
            ) {
                Ok(()) => return Ok(()),
                Err(error) => {
                    let retryable = error
                        .split_whitespace()
                        .nth(1)
                        .and_then(|value| value.parse::<u16>().ok())
                        .and_then(|code| StatusCode::from_u16(code).ok())
                        .is_some_and(is_retryable_status)
                        || error.starts_with("Network error");
                    failures.push(error);
                    if !retryable || attempt == HTTP_ATTEMPTS {
                        break;
                    }
                    sleep(retry_delay(attempt));
                }
            }
        }
    }
    Err(failures
        .last()
        .cloned()
        .unwrap_or_else(|| "Download failed without a response.".to_string()))
}

fn git_blob_sha1(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha1::new();
    hasher.update(format!("blob {}\0", bytes.len()).as_bytes());
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn download_source_blob_fallback(
    app: &AppHandle,
    client: &Client,
    expected_blob: &str,
    destination: &Path,
    overall_start: f64,
    overall_end: f64,
) -> Result<(), String> {
    emit_install(
        app,
        "source",
        0.0,
        overall_start,
        "Raw source delivery failed; retrying the same pinned file through GitHub's blob API",
        None,
        None,
    );
    let url = format!(
        "https://api.github.com/repos/VAST-AI-Research/TripoSR/git/blobs/{expected_blob}"
    );
    let mut last_error = None;
    for attempt in 1..=HTTP_ATTEMPTS {
        match client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
        {
            Ok(response) if response.status().is_success() => {
                let text = response
                    .text()
                    .map_err(|error| format!("Could not read GitHub blob fallback response: {error}"))?;
                let payload = serde_json::from_str::<GitBlobResponse>(&text)
                    .map_err(|error| format!("GitHub blob fallback returned invalid JSON: {error}"))?;
                if payload.sha != expected_blob || payload.encoding != "base64" {
                    return Err("GitHub blob fallback did not match the expected pinned object.".to_string());
                }
                let compact = payload
                    .content
                    .chars()
                    .filter(|ch| !ch.is_whitespace())
                    .collect::<String>();
                let bytes = BASE64
                    .decode(compact.as_bytes())
                    .map_err(|error| format!("Could not decode the pinned GitHub blob: {error}"))?;
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                fs::write(destination, bytes).map_err(|error| error.to_string())?;
                if git_blob_sha1(destination)? != expected_blob {
                    let _ = fs::remove_file(destination);
                    return Err("GitHub blob fallback failed source integrity verification.".to_string());
                }
                emit_install(
                    app,
                    "source",
                    1.0,
                    overall_end,
                    "Pinned source recovered through GitHub's blob API",
                    None,
                    None,
                );
                return Ok(());
            }
            Ok(response) => {
                let status = response.status();
                last_error = Some(format!("GitHub blob API returned HTTP {status}."));
                if !is_retryable_status(status) {
                    break;
                }
            }
            Err(error) => last_error = Some(format!("GitHub blob API network error: {error}")),
        }
        if attempt < HTTP_ATTEMPTS {
            sleep(retry_delay(attempt));
        }
    }
    Err(last_error.unwrap_or_else(|| "GitHub blob fallback failed.".to_string()))
}

fn download_verified_source(
    app: &AppHandle,
    client: &Client,
    relative: &str,
    expected_blob: &str,
    destination: &Path,
    overall_start: f64,
    overall_end: f64,
) -> Result<(), String> {
    let raw_url = format!(
        "https://raw.githubusercontent.com/VAST-AI-Research/TripoSR/{TRIPOSR_SOURCE_REVISION}/{relative}"
    );
    let result = download_http_with_retries(
        app,
        client,
        &[raw_url],
        destination,
        "source",
        overall_start,
        overall_end,
        "Downloading pinned TripoSR source",
    );
    if result.is_ok() && git_blob_sha1(destination)? == expected_blob {
        return Ok(());
    }
    let _ = fs::remove_file(destination);
    download_source_blob_fallback(
        app,
        client,
        expected_blob,
        destination,
        overall_start,
        overall_end,
    )
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

fn install_blocking(app: AppHandle, alternate_model_url: Option<String>) -> Result<ModelRuntimeState, String> {
    let alternate_model_url = alternate_model_url
        .filter(|value| !value.trim().is_empty())
        .map(|value| validate_https_url(&value))
        .transpose()?;

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
            "--retries",
            "5",
            "--timeout",
            "60",
        ];
        pip_args.extend(PYTHON_REQUIREMENTS.iter().copied());
        emit_install(&app, "runtime", 0.25, 0.05, "Installing pinned model dependencies", None, None);
        let status = Command::new(&python)
            .args(pip_args)
            .status()
            .map_err(|error| error.to_string())?;
        if !status.success() {
            return Err("Could not install the pinned TripoSR Python dependencies after pip retries.".into());
        }
        emit_install(&app, "runtime", 1.0, 0.23, "Private runtime ready", None, None);

        let client = Client::builder()
            .user_agent(format!("Still2Solid/{}", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|error| error.to_string())?;
        let source_root = staging.join("source");
        for (index, (relative, expected_blob)) in SOURCE_FILES.iter().enumerate() {
            let destination = source_root.join(relative);
            let start = 0.23 + index as f64 / SOURCE_FILES.len() as f64 * 0.12;
            let end = 0.23 + (index + 1) as f64 / SOURCE_FILES.len() as f64 * 0.12;
            download_verified_source(
                &app,
                &client,
                relative,
                expected_blob,
                &destination,
                start,
                end,
            )?;
        }
        emit_install(&app, "source", 1.0, 0.35, "Pinned TripoSR source verified", None, None);

        let official_urls = vec![
            format!("https://huggingface.co/stabilityai/TripoSR/resolve/{TRIPOSR_WEIGHT_REVISION}/model.ckpt?download=true"),
            format!("https://huggingface.co/stabilityai/TripoSR/resolve/{TRIPOSR_WEIGHT_REVISION}/model.ckpt"),
        ];
        let model_urls = alternate_model_url
            .as_ref()
            .map(|url| vec![url.clone()])
            .unwrap_or(official_urls);
        let model = staging.join("model.ckpt");
        let model_message = if alternate_model_url.is_some() {
            "Downloading TripoSR checkpoint from the user-supplied recovery link"
        } else {
            "Downloading the pinned TripoSR checkpoint"
        };
        download_http_with_retries(
            &app,
            &client,
            &model_urls,
            &model,
            "weights",
            0.35,
            0.79,
            model_message,
        )
        .map_err(|error| {
            if alternate_model_url.is_some() {
                format!("Alternate model download failed: {error}")
            } else {
                format!("Automatic TripoSR model download failed after retries and fallback URL: {error}")
            }
        })?;
        if sha256_file(&model)? != TRIPOSR_WEIGHT_SHA256 {
            let _ = fs::remove_file(&model);
            return Err("TripoSR checkpoint SHA-256 verification failed. Still2Solid removed the untrusted file.".into());
        }
        emit_install(&app, "weights", 1.0, 0.79, "TripoSR checkpoint verified", None, None);

        let u2net = staging.join("u2net.onnx");
        download_http_with_retries(
            &app,
            &client,
            &["https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx".to_string()],
            &u2net,
            "foreground",
            0.79,
            0.91,
            "Downloading foreground-isolation support",
        )?;
        if md5_file(&u2net)? != U2NET_MD5 {
            let _ = fs::remove_file(&u2net);
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
            "sourceManifest": "verified-upstream-layout-v2",
            "modelDownloadMode": if alternate_model_url.is_some() { "verified-recovery-url" } else { "official-pinned" }
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
    model_url: Option<String>,
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
        install_blocking(app, model_url)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::{retry_delay, validate_https_url, SOURCE_FILES};

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

    #[test]
    fn recovery_model_url_must_be_https_without_credentials() {
        assert!(validate_https_url("https://example.com/model.ckpt").is_ok());
        assert!(validate_https_url("http://example.com/model.ckpt").is_err());
        assert!(validate_https_url("https://user:pass@example.com/model.ckpt").is_err());
    }

    #[test]
    fn retry_delay_increases_but_stays_bounded() {
        assert!(retry_delay(2) > retry_delay(1));
        assert_eq!(retry_delay(10), retry_delay(4));
    }
}
