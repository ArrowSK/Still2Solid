use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSummary {
    models_bytes: u64,
    cache_bytes: u64,
    temporary_bytes: u64,
    other_app_data_bytes: u64,
    total_removable_bytes: u64,
    installed_model_directories: u64,
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

fn unique_top_level_roots(mut roots: Vec<PathBuf>) -> Vec<PathBuf> {
    roots.sort_by_key(|path| path.components().count());
    let mut unique = Vec::<PathBuf>::new();
    for root in roots {
        if unique.iter().any(|existing| root == *existing || root.starts_with(existing)) {
            continue;
        }
        unique.push(root);
    }
    unique
}

fn owned_roots(app: &AppHandle) -> Result<Vec<PathBuf>, String> {
    let resolver = app.path();
    let roots = vec![
        resolver.app_data_dir().map_err(|error| error.to_string())?,
        resolver.app_cache_dir().map_err(|error| error.to_string())?,
        resolver.app_config_dir().map_err(|error| error.to_string())?,
        resolver.app_local_data_dir().map_err(|error| error.to_string())?,
    ];
    Ok(unique_top_level_roots(roots))
}

fn validate_owned_root(path: &Path) -> Result<(), String> {
    let leaf = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !leaf.contains("still2solid") {
        return Err(format!(
            "Still2Solid refused to remove an unexpected application-data path: {}",
            path.display()
        ));
    }
    Ok(())
}

fn validate_owned_child(parent: &Path, child: &Path) -> Result<(), String> {
    if child == parent || !child.starts_with(parent) {
        return Err(format!(
            "Still2Solid refused to remove an unexpected temporary path: {}",
            child.display()
        ));
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|error| format!("Could not remove {}: {error}", path.display()))
    } else {
        fs::remove_file(path).map_err(|error| format!("Could not remove {}: {error}", path.display()))
    }
}

fn abandoned_paths(app: &AppHandle) -> Result<Vec<PathBuf>, String> {
    let app_data = app.path().app_data_dir().map_err(|error| error.to_string())?;
    let mut paths = vec![app_data.join("jobs")];
    let models = app_data.join("models");
    if let Ok(entries) = fs::read_dir(&models) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let is_installing = path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.ends_with(".installing"));
            if is_installing {
                paths.push(path);
            }
        }
    }
    for path in &paths {
        validate_owned_child(&app_data, path)?;
    }
    Ok(paths)
}

fn abandoned_size(app: &AppHandle) -> Result<u64, String> {
    Ok(abandoned_paths(app)?
        .iter()
        .map(|path| directory_size(path))
        .sum())
}

pub fn cleanup_abandoned_work(app: &AppHandle) -> Result<u64, String> {
    let paths = abandoned_paths(app)?;
    let bytes = paths.iter().map(|path| directory_size(path)).sum();
    for path in paths {
        remove_path(&path)?;
    }
    Ok(bytes)
}

fn summary_for(app: &AppHandle) -> Result<StorageSummary, String> {
    let resolver = app.path();
    let app_data = resolver.app_data_dir().map_err(|error| error.to_string())?;
    let cache = resolver.app_cache_dir().map_err(|error| error.to_string())?;
    let models = app_data.join("models");

    let temporary_bytes = abandoned_size(app)?;
    let raw_models_bytes = directory_size(&models);
    let staging_bytes = abandoned_paths(app)?
        .iter()
        .filter(|path| path.starts_with(&models))
        .map(|path| directory_size(path))
        .sum::<u64>();
    let models_bytes = raw_models_bytes.saturating_sub(staging_bytes);
    let cache_bytes = directory_size(&cache);
    let total_removable_bytes = owned_roots(app)?
        .iter()
        .map(|root| directory_size(root))
        .sum::<u64>();
    let other_app_data_bytes = total_removable_bytes
        .saturating_sub(models_bytes)
        .saturating_sub(cache_bytes)
        .saturating_sub(temporary_bytes);
    let installed_model_directories = fs::read_dir(&models)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| {
            let path = entry.path();
            path.is_dir()
                && !path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|name| name.ends_with(".installing"))
        })
        .count() as u64;

    Ok(StorageSummary {
        models_bytes,
        cache_bytes,
        temporary_bytes,
        other_app_data_bytes,
        total_removable_bytes,
        installed_model_directories,
    })
}

#[tauri::command]
pub fn get_storage_summary(app: AppHandle) -> Result<StorageSummary, String> {
    summary_for(&app)
}

#[tauri::command]
pub fn clear_app_cache(app: AppHandle) -> Result<StorageSummary, String> {
    let resolver = app.path();
    let app_data = resolver.app_data_dir().map_err(|error| error.to_string())?;
    let cache = resolver.app_cache_dir().map_err(|error| error.to_string())?;

    if cache == app_data || app_data.starts_with(&cache) {
        return Err("Still2Solid refused to clear a cache path that overlaps the application-data root.".into());
    }
    validate_owned_root(&cache)?;
    cleanup_abandoned_work(&app)?;
    remove_path(&cache)?;
    summary_for(&app)
}

#[tauri::command]
pub fn clear_app_owned_data(app: AppHandle) -> Result<StorageSummary, String> {
    let roots = owned_roots(&app)?;
    for root in &roots {
        validate_owned_root(root)?;
    }
    for root in roots {
        remove_path(&root)?;
    }
    summary_for(&app)
}

#[tauri::command]
pub fn open_applications_folder() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("/Applications")
            .spawn()
            .map_err(|error| format!("Could not open Applications: {error}"))?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("Open Applications is only available in the macOS build.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::unique_top_level_roots;
    use std::path::PathBuf;

    #[test]
    fn removes_duplicate_and_nested_roots() {
        let roots = vec![
            PathBuf::from("/tmp/still2solid"),
            PathBuf::from("/tmp/still2solid/cache"),
            PathBuf::from("/tmp/still2solid"),
            PathBuf::from("/tmp/still2solid-local"),
        ];
        let result = unique_top_level_roots(roots);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&PathBuf::from("/tmp/still2solid")));
        assert!(result.contains(&PathBuf::from("/tmp/still2solid-local")));
    }
}
