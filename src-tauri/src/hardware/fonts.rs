use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use ab_glyph::{FontArc, FontVec};

use super::HardwareError;

#[derive(Clone)]
struct InstalledFont {
    path: PathBuf,
    index: u32,
}

static CATALOG: OnceLock<Mutex<BTreeMap<String, InstalledFont>>> = OnceLock::new();

fn font_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(windows) = std::env::var_os("WINDIR") {
        directories.push(PathBuf::from(windows).join("Fonts"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        directories.push(PathBuf::from(local).join("Microsoft/Windows/Fonts"));
    }
    directories
}

fn scan_fonts() -> BTreeMap<String, InstalledFont> {
    let mut fonts = BTreeMap::new();
    for directory in font_directories() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !matches!(extension.as_str(), "ttf" | "otf" | "ttc" | "otc") {
                continue;
            }
            let Ok(bytes) = fs::read(&path) else { continue };
            for index in 0..ttf_parser::fonts_in_collection(&bytes).unwrap_or(1) {
                let Ok(face) = ttf_parser::Face::parse(&bytes, index) else {
                    continue;
                };
                // The label contains Thai captions even when patient/drug names are Latin.
                if !('ก'..='ฺ')
                    .chain('฿'..='๛')
                    .chain("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".chars())
                    .all(|character| face.glyph_index(character).is_some())
                {
                    continue;
                }
                let Some(name) = face
                    .names()
                    .into_iter()
                    .filter(|name| name.name_id == ttf_parser::name_id::FULL_NAME)
                    .filter_map(|name| name.to_string())
                    .find(|name| !name.trim().is_empty())
                else {
                    continue;
                };
                if FontVec::try_from_vec_and_index(bytes.clone(), index).is_ok() {
                    fonts.entry(name).or_insert(InstalledFont {
                        path: path.clone(),
                        index,
                    });
                }
            }
        }
    }
    fonts
}

pub(crate) fn list_system_label_fonts() -> Result<Vec<String>, HardwareError> {
    let discovered = scan_fonts();
    let names = discovered.keys().cloned().collect();
    *CATALOG
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| HardwareError::FontUnavailable)? = discovered;
    Ok(names)
}

pub(crate) fn load_selected_font(name: &str) -> Result<FontArc, HardwareError> {
    // Resolve only discovered names, never paths supplied by the UI or a remote peer.
    let mut catalog = CATALOG
        .get_or_init(|| Mutex::new(scan_fonts()))
        .lock()
        .map_err(|_| HardwareError::FontUnavailable)?;
    if !catalog.contains_key(name) {
        *catalog = scan_fonts();
    }
    let installed = catalog.get(name).ok_or(HardwareError::FontUnavailable)?;
    let bytes = fs::read(&installed.path).map_err(|_| HardwareError::FontUnavailable)?;
    FontVec::try_from_vec_and_index(bytes, installed.index)
        .map(FontArc::new)
        .map_err(|_| HardwareError::FontUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ab_glyph::Font;

    #[test]
    fn arbitrary_paths_and_unknown_names_cannot_be_loaded() {
        assert!(load_selected_font("../../oncoflow.db").is_err());
        assert!(load_selected_font("OncoFlow nonexistent font").is_err());
    }

    #[test]
    #[cfg(windows)]
    fn discovers_and_loads_local_thai_fonts() {
        let names = list_system_label_fonts().unwrap();
        assert!(!names.is_empty());
        let font = load_selected_font(&names[0]).unwrap();
        assert_ne!(font.glyph_id('ก').0, 0);
    }
}
