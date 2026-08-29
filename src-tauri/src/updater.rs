use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Emitter, Manager};

const RELEASE_API: &str = "https://api.github.com/repos/ArrowSK/Still2Solid/releases/latest";
const RELEASE_REPOSITORY: &str = "ArrowSK/Still2Solid";

#[derive(Debug, Clone, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
    size: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    available: bool,
    current_version: String,
    latest_version: String,
    release_url: String,
    notes: String,
    download_size: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadedUpdate {
    version: String,
    path: String,
    sha256: String,
    release_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateDownloadProgress {
    downloaded_bytes: u64,
    total_bytes: u64,
    progress: f64,
    message: String,
}

fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent(format!("Still2Solid/{} ({RELEASE_REPOSITORY})", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("Could not prepare the updater: {error}"))
}

fn normalized_version(value: &str) -> Option<Vec<u64>> {
    let value = value.trim().trim_start_matches(['v', 'V']);
    let core = value.split(['-', '+']).next()?;
    let parts = core
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;
    (!parts.is_empty()).then_some(parts)
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    let Some(mut latest) = normalized_version(latest) else {
        return false;
    };
    let Some(mut current) = normalized_version(current) else {
        return false;
    };
    let len = latest.len().max(current.len());
    latest.resize(len, 0);
    current.resize(len, 0);
    latest > current
}

fn latest_release(client: &Client) -> Result<GitHubRelease, String> {
    let response = client
        .get(RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .map_err(|error| format!("Could not check GitHub for updates: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "GitHub update check failed with HTTP {}.",
            response.status()
        ));
    }
    let text = response
        .text()
        .map_err(|error| format!("Could not read the GitHub release response: {error}"))?;
    serde_json::from_str::<GitHubRelease>(&text)
        .map_err(|error| format!("GitHub returned an unreadable release response: {error}"))
}

fn macos_asset(release: &GitHubRelease) -> Option<&GitHubReleaseAsset> {
    release.assets.iter().find(|asset| {
        asset.name.starts_with("Still2Solid_") && asset.name.ends_with("_aarch64.dmg")
    })
}

fn update_info_for(release: &GitHubRelease) -> UpdateInfo {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let latest = release.tag_name.trim_start_matches(['v', 'V']).to_string();
    UpdateInfo {
        available: cfg!(all(target_os = "macos", target_arch = "aarch64"))
            && version_is_newer(&latest, &current)
            && macos_asset(release).is_some(),
        current_version: current,
        latest_version: latest,
        release_url: release.html_url.clone(),
        notes: release.body.clone().unwrap_or_default(),
        download_size: macos_asset(release).map(|asset| asset.size).unwrap_or(0),
    }
}

fn checksum_from_release(
    client: &Client,
    release: &GitHubRelease,
    asset: &GitHubReleaseAsset,
) -> Result<String, String> {
    if let Some(value) = asset.digest.as_deref().and_then(|value| value.strip_prefix("sha256:")) {
        if value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Ok(value.to_ascii_lowercase());
        }
    }

    let checksum_asset = release
        .assets
        .iter()
        .find(|candidate| candidate.name == "Still2Solid-macOS-Apple-Silicon.sha256.txt")
        .ok_or_else(|| "The release does not provide a SHA-256 checksum for the macOS installer.".to_string())?;
    let response = client
        .get(&checksum_asset.browser_download_url)
        .send()
        .map_err(|error| format!("Could not download the release checksum: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Checksum download failed with HTTP {}.",
            response.status()
        ));
    }
    let text = response
        .text()
        .map_err(|error| format!("Could not read the release checksum: {error}"))?;
    for line in text.lines() {
        if line.contains(&asset.name) {
            if let Some(value) = line.split_whitespace().next() {
                if value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
                    return Ok(value.to_ascii_lowercase());
                }
            }
        }
    }
    Err("The release checksum file does not contain the selected macOS installer.".to_string())
}

fn update_cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?
        .join("updates"))
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

fn emit_progress(app: &AppHandle, downloaded: u64, total: u64, message: &str) {
    let progress = if total > 0 {
        (downloaded as f64 / total as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let _ = app.emit(
        "update-download-progress",
        UpdateDownloadProgress {
            downloaded_bytes: downloaded,
            total_bytes: total,
            progress,
            message: message.to_string(),
        },
    );
}

fn download_latest(app: AppHandle) -> Result<DownloadedUpdate, String> {
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        let _ = app;
        return Err("Automatic Still2Solid installer download is currently available for Apple Silicon macOS builds.".to_string());
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        let client = client()?;
        let release = latest_release(&client)?;
        let info = update_info_for(&release);
        if !info.available {
            return Err(format!("Still2Solid {} is already up to date.", info.current_version));
        }
        let asset = macos_asset(&release)
            .ok_or_else(|| "The latest release does not contain an Apple Silicon DMG.".to_string())?;
        let expected_sha256 = checksum_from_release(&client, &release, asset)?;
        let cache = update_cache_dir(&app)?;
        fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
        let destination = cache.join(&asset.name);
        let temporary = destination.with_extension("dmg.part");
        if temporary.exists() {
            let _ = fs::remove_file(&temporary);
        }

        emit_progress(&app, 0, asset.size, "Downloading the verified update from GitHub");
        let mut response = client
            .get(&asset.browser_download_url)
            .send()
            .map_err(|error| format!("Update download failed: {error}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "Update download failed with HTTP {}.",
                response.status()
            ));
        }
        let total = response.content_length().unwrap_or(asset.size);
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
            emit_progress(&app, downloaded, total, "Downloading the verified update from GitHub");
        }
        file.flush().map_err(|error| error.to_string())?;

        let actual_sha256 = sha256_file(&temporary)?;
        if actual_sha256 != expected_sha256 {
            let _ = fs::remove_file(&temporary);
            return Err("The downloaded update failed SHA-256 verification and was removed.".to_string());
        }
        if destination.exists() {
            fs::remove_file(&destination).map_err(|error| error.to_string())?;
        }
        fs::rename(&temporary, &destination).map_err(|error| error.to_string())?;
        emit_progress(&app, total, total, "Update verified and ready to install");

        Ok(DownloadedUpdate {
            version: info.latest_version,
            path: destination.to_string_lossy().to_string(),
            sha256: actual_sha256,
            release_url: release.html_url,
        })
    }
}

#[tauri::command]
pub async fn check_for_updates() -> Result<UpdateInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let client = client()?;
        let release = latest_release(&client)?;
        Ok(update_info_for(&release))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn download_update(app: AppHandle) -> Result<DownloadedUpdate, String> {
    tauri::async_runtime::spawn_blocking(move || download_latest(app))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn open_update_installer(app: AppHandle, path: String) -> Result<(), String> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        let _ = path;
        return Err("The DMG installer flow is only available on macOS.".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let root = update_cache_dir(&app)?;
        let root = fs::canonicalize(root).map_err(|error| error.to_string())?;
        let candidate = fs::canonicalize(PathBuf::from(path)).map_err(|error| error.to_string())?;
        if !candidate.starts_with(&root) || candidate.extension().and_then(|value| value.to_str()) != Some("dmg") {
            return Err("Still2Solid refused to open an installer outside its verified update cache.".to_string());
        }
        Command::new("open")
            .arg(&candidate)
            .spawn()
            .map_err(|error| format!("Could not open the macOS installer: {error}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{normalized_version, version_is_newer};

    #[test]
    fn normalizes_release_tags() {
        assert_eq!(normalized_version("v0.8.3"), Some(vec![0, 8, 3]));
        assert_eq!(normalized_version("1.2.0+build"), Some(vec![1, 2, 0]));
    }

    #[test]
    fn compares_versions_numerically() {
        assert!(version_is_newer("0.10.0", "0.9.9"));
        assert!(version_is_newer("v0.8.3", "0.8.2"));
        assert!(!version_is_newer("0.8.2", "0.8.2"));
        assert!(!version_is_newer("0.8.1", "0.8.2"));
    }
}
