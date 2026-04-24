//! Persistent cache for resolved station stream URLs.
//!
//! Stored as a JSON object mapping station name → URL string in the XDG data dir.
use crate::cli::dir_strategy;
use crate::stations::{ClassicalStations, Station};
use anyhow::Result;
use clap::ValueEnum;
use etcetera::AppStrategy;
use std::collections::HashMap;
use url::Url;

fn cache_path() -> Result<std::path::PathBuf> {
    let dir = dir_strategy()?.data_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("station_cache.json"))
}

/// Load the cache from disk, returning a map of Station → Url.
/// Returns an empty map on any error (missing file, corrupt JSON, etc.).
pub fn load() -> HashMap<Station, Url> {
    load_inner().unwrap_or_default()
}

fn load_inner() -> Result<HashMap<Station, Url>> {
    let path = cache_path()?;
    let raw = std::fs::read_to_string(path)?;
    let map: HashMap<String, String> = serde_json::from_str(&raw)?;

    let mut result = HashMap::new();
    for variant in ClassicalStations::value_variants() {
        let station = variant.station();
        if let Some(url_str) = map.get(station.name)
            && let Ok(url) = Url::parse(url_str)
        {
            result.insert(station, url);
        }
    }
    Ok(result)
}

/// Persist a resolved URL for a station.
pub fn save(station: Station, url: &Url) -> Result<()> {
    let mut map = load_raw()?;
    map.insert(station.name.to_string(), url.to_string());
    write_raw(&map)
}

/// Remove a station's entry (e.g. after a connection failure).
pub fn invalidate(station: Station) -> Result<()> {
    let mut map = load_raw()?;
    if map.remove(station.name).is_some() {
        write_raw(&map)?;
    }
    Ok(())
}

/// Delete the entire cache file.
pub fn clear() -> Result<()> {
    let path = cache_path()?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn load_raw() -> Result<HashMap<String, String>> {
    let path = cache_path()?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn write_raw(map: &HashMap<String, String>) -> Result<()> {
    let path = cache_path()?;
    let json = serde_json::to_string_pretty(map)?;
    std::fs::write(path, json)?;
    Ok(())
}
