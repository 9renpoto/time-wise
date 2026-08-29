use std::fs;

#[test]
fn application_versions_are_consistent() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let backend_manifest =
        fs::read_to_string(format!("{manifest_dir}/Cargo.toml")).expect("read backend Cargo.toml");
    let backend: toml::Value = toml::from_str(&backend_manifest).expect("parse backend Cargo.toml");

    let ui_manifest =
        fs::read_to_string(format!("{manifest_dir}/../Cargo.toml")).expect("read UI Cargo.toml");
    let ui: toml::Value = toml::from_str(&ui_manifest).expect("parse UI Cargo.toml");

    let tauri_config = fs::read_to_string(format!("{manifest_dir}/tauri.conf.json"))
        .expect("read tauri.conf.json");
    let tauri: serde_json::Value =
        serde_json::from_str(&tauri_config).expect("parse tauri.conf.json");

    let backend_version = backend["package"]["version"]
        .as_str()
        .expect("backend package version");
    assert_eq!(
        ui["package"]["version"].as_str(),
        Some(backend_version),
        "UI and backend package versions must match"
    );
    assert_eq!(
        tauri["version"].as_str(),
        Some(backend_version),
        "Tauri and backend package versions must match"
    );
}

#[test]
fn tauri_window_defaults_and_build_commands() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let conf_path = format!("{}/tauri.conf.json", manifest_dir);
    let data = fs::read_to_string(conf_path).expect("read tauri.conf.json");
    let v: serde_json::Value = serde_json::from_str(&data).expect("parse json");

    // product identity
    assert_eq!(v["productName"], "Time Wise");
    assert_eq!(v["identifier"], "io.github.umeno3.time-wise");

    // build commands
    assert_eq!(v["build"]["beforeDevCommand"], "trunk serve --no-color");
    assert_eq!(v["build"]["beforeBuildCommand"], "trunk build --no-color");
    assert_eq!(v["build"]["devUrl"], "http://localhost:1420");

    // window defaults
    let win0 = &v["app"]["windows"][0];
    assert_eq!(win0["title"], "Time Wise");
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
        "Win32_Security_Credentials",
        "Win32_System_Power",
        "Win32_System_RemoteDesktop",
        "Win32_System_Threading",
        "Win32_UI_Accessibility",
        "Win32_UI_WindowsAndMessaging",
    ] {
        assert!(features.contains(&required), "missing feature: {required}");
    }

    let windows_dependencies = &manifest["target"]["cfg(target_os = \"windows\")"]["dependencies"];
    assert!(
        windows_dependencies.get("keyring").is_none(),
        "Windows credentials must use an explicit local-machine persistence policy"
    );
    assert_eq!(
        manifest["target"]["cfg(target_os = \"macos\")"]["dependencies"]["keyring"]["features"][0]
            .as_str(),
        Some("apple-native")
    );
}

#[test]
fn packaged_build_enables_tauri_custom_protocol() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let manifest_path = format!("{manifest_dir}/Cargo.toml");
    let data = fs::read_to_string(manifest_path).expect("read Cargo.toml");
    let manifest: toml::Value = toml::from_str(&data).expect("parse Cargo.toml");

    let custom_protocol = manifest["features"]["custom-protocol"]
        .as_array()
        .expect("custom-protocol feature")
        .iter()
        .filter_map(toml::Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(custom_protocol, ["tauri/custom-protocol"]);
}

#[test]
fn backend_enforces_a_single_desktop_instance() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let manifest_path = format!("{manifest_dir}/Cargo.toml");
    let data = fs::read_to_string(manifest_path).expect("read Cargo.toml");
    let manifest: toml::Value = toml::from_str(&data).expect("parse Cargo.toml");

    assert_eq!(
        manifest["dependencies"]["tauri-plugin-single-instance"].as_str(),
        Some("2")
    );

    let lib_path = format!("{manifest_dir}/src/lib.rs");
    let lib = fs::read_to_string(lib_path).expect("read backend lib.rs");
    let single_instance = lib
        .find(".plugin(tauri_plugin_single_instance::init")
        .expect("single-instance plugin registration");
    let opener = lib
        .find(".plugin(tauri_plugin_opener::init")
        .expect("opener plugin registration");
    let autostart = lib
        .find("tauri_plugin_autostart::Builder::new()")
        .expect("autostart plugin registration");

    assert!(single_instance < opener);
    assert!(single_instance < autostart);
    assert!(lib.contains("const AUTOSTART_APP_NAME: &str = \"time-wise\";"));
    assert!(lib.contains(".app_name(AUTOSTART_APP_NAME)"));
}
