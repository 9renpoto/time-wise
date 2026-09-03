use std::fs;

#[test]
fn tray_icon_png_exists_and_non_empty() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let icon_path = format!("{}/icons/32x32.png", manifest_dir);
    let data = fs::read(icon_path).expect("read tray icon 32x32.png");
    assert!(!data.is_empty(), "tray icon should not be empty");
}

#[test]
fn cargo_tauri_features_include_image_png_and_tray_icon() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cargo_toml_path = format!("{}/Cargo.toml", manifest_dir);
    let data = fs::read_to_string(cargo_toml_path).expect("read Cargo.toml");
    let v: toml::Value = toml::from_str(&data).expect("parse Cargo.toml");

    let features = v["dependencies"]["tauri"]["features"]
        .as_array()
        .expect("tauri features should be an array")
        .iter()
        .filter_map(|x| x.as_str())
        .collect::<Vec<_>>();

    assert!(features.contains(&"tray-icon"), "missing tray-icon feature");
    assert!(features.contains(&"image-png"), "missing image-png feature");
}
