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
    assert_eq!(win0["decorations"], false);
    assert_eq!(win0["skipTaskbar"], true);
    assert_eq!(win0["width"], 400);
    assert_eq!(win0["height"], 300);
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
