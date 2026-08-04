use std::fs;

#[test]
fn tauri_window_defaults_and_build_commands() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let conf_path = format!("{}/tauri.conf.json", manifest_dir);
    let data = fs::read_to_string(conf_path).expect("read tauri.conf.json");
    let v: serde_json::Value = serde_json::from_str(&data).expect("parse json");

    // build commands
    assert_eq!(v["build"]["beforeDevCommand"], "trunk serve");
    assert_eq!(v["build"]["beforeBuildCommand"], "trunk build");
    assert_eq!(v["build"]["devUrl"], "http://localhost:1420");

    // window defaults
    let win0 = &v["app"]["windows"][0];
    assert_eq!(win0["visible"], false);
    assert_eq!(win0["decorations"], true);
    assert_eq!(win0["skipTaskbar"], true);
    assert_eq!(win0["width"], 900);
    assert_eq!(win0["height"], 700);
    assert_eq!(win0["minWidth"], 440);
    assert_eq!(win0["minHeight"], 520);
}

#[test]
fn trunk_config_serving_and_bindgen_version() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // Trunk.toml はワークスペースルート上
    let trunk_path = format!("{}/../Trunk.toml", manifest_dir);
    let data = fs::read_to_string(trunk_path).expect("read Trunk.toml");
    let v: toml::Value = toml::from_str(&data).expect("parse toml");

    // [build]
    assert_eq!(v["build"]["target"].as_str(), Some("./index.html"));
    assert_eq!(v["build"]["wasm-bindgen"].as_str(), Some("0.2.92"));

    // [serve]
    assert_eq!(v["serve"]["port"].as_integer(), Some(1420));
    assert_eq!(v["serve"]["open"].as_bool(), Some(false));
}

#[test]
fn windows_api_dependency_is_target_specific() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let manifest_path = format!("{}/Cargo.toml", manifest_dir);
    let data = fs::read_to_string(manifest_path).expect("read Cargo.toml");
    let manifest: toml::Value = toml::from_str(&data).expect("parse Cargo.toml");

    assert!(manifest["dependencies"].get("windows").is_none());

    let windows = &manifest["target"]["cfg(target_os = \"windows\")"]["dependencies"]["windows"];
    assert_eq!(windows["version"].as_str(), Some("0.62"));

    let features = windows["features"]
        .as_array()
        .expect("windows features")
        .iter()
        .filter_map(toml::Value::as_str)
        .collect::<Vec<_>>();
    for required in [
        "Win32_System_Power",
        "Win32_System_RemoteDesktop",
        "Win32_System_Threading",
        "Win32_UI_Accessibility",
        "Win32_UI_WindowsAndMessaging",
    ] {
        assert!(features.contains(&required), "missing feature: {required}");
    }
}
