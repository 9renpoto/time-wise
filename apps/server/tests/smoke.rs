#![cfg(not(target_arch = "wasm32"))]

mod support;

#[test]
#[ignore = "requires npm ci and Node; local D1 only, no Cloudflare credentials"]
fn deployment_migrations_preserve_existing_data_on_repeated_setup() {
    use serde_json::json;
    use std::{fs, process::Command, time::Duration};
    let project = tempfile::tempdir().unwrap();
    let migrations = project.path().join("migrations");
    fs::create_dir(&migrations).unwrap();
    fs::write(
        migrations.join("0001_synthetic.sql"),
        "CREATE TABLE synthetic_records (id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
    )
    .unwrap();
    fs::write(
        project.path().join("wrangler.json"),
        json!({
            "name":"synthetic-deployment-check", "compatibility_date":"2026-10-29",
            "d1_databases":[{"binding":"AUTH_DB", "database_name":"operator-selected-name", "database_id":"11111111-2222-3333-4444-555555555555", "migrations_dir":"migrations"}]
        }).to_string(),
    ).unwrap();
    let run = |arguments: &[&str]| {
        support::run(
            Command::new("node")
                .arg(support::root().join("node_modules/wrangler/bin/wrangler.js"))
                .args(arguments)
                .args(["--local", "--config", "wrangler.json"])
                .current_dir(project.path())
                .env("WRANGLER_SEND_METRICS", "false")
                .env("CI", "true"),
            Duration::from_secs(60),
        )
        .unwrap()
    };
    run(&["d1", "migrations", "apply", "AUTH_DB"]);
    run(&[
        "d1",
        "execute",
        "AUTH_DB",
        "--command",
        "INSERT INTO synthetic_records VALUES (1, 'retained');",
    ]);
    run(&["d1", "migrations", "apply", "AUTH_DB"]);
    fs::write(
        migrations.join("0002_synthetic.sql"),
        "CREATE INDEX synthetic_value ON synthetic_records(value);",
    )
    .unwrap();
    run(&["d1", "migrations", "apply", "AUTH_DB"]);
    let result: serde_json::Value = serde_json::from_str(&run(&[
        "d1",
        "execute",
        "AUTH_DB",
        "--command",
        "SELECT value FROM synthetic_records WHERE id=1;",
        "--json",
    ]))
    .unwrap();
    assert_eq!(result[0]["results"], json!([{"value":"retained"}]));
}

#[cfg(unix)]
#[test]
fn failed_commands_report_failure() {
    let result = support::run(
        std::process::Command::new("sh").args(["-c", "exit 7"]),
        std::time::Duration::from_secs(2),
    );
    assert!(result.is_err());
}

#[cfg(unix)]
#[test]
fn stalled_commands_are_bounded() {
    let start = std::time::Instant::now();
    let result = support::run(
        std::process::Command::new("sh").args(["-c", "exec sleep 10"]),
        std::time::Duration::from_millis(30),
    );
    assert!(result.is_err());
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
}

#[test]
fn successful_commands_capture_output() {
    assert_eq!(
        support::run(
            std::process::Command::new("rustc").arg("--version"),
            std::time::Duration::from_secs(5)
        )
        .unwrap()
        .split_whitespace()
        .next(),
        Some("rustc")
    );
}

#[test]
#[ignore = "requires Docker and worker-build; run explicitly with coverage in CI"]
fn worker_discovery_and_fail_closed_routes() {
    use reqwest::Method;
    use serde_json::json;
    support::build("apps/server/scripts/build.sh");
    let mut sandbox = support::Sandbox::new();
    for file in ["index.js", "index_bg.wasm"] {
        sandbox.copy(
            &support::root().join("apps/server/build").join(file),
            &format!("build/{file}"),
        );
    }
    let mut config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(support::root().join("apps/server/wrangler.jsonc")).unwrap(),
    )
    .unwrap();
    config.as_object_mut().unwrap().remove("build");
    sandbox.configure(&config);
    let base = sandbox.start();
    sandbox.wait(&base, "/v1/auth/config", |status, _| status == 200);
    let (status, body, headers) = sandbox
        .request(&base, "/v1/auth/config", Method::GET)
        .unwrap();
    assert_eq!(status, 200);
    assert_eq!(
        body,
        json!({"protocol_version":1,"auth_origin":"https://auth.example.com","client_profile_ids":["official-development-v1"]})
    );
    assert_eq!(headers["cache-control"], "no-store");
    assert!(!headers.contains_key("access-control-allow-origin"));
    let (status, _, headers) = sandbox
        .request(&base, "/v1/auth/config", Method::POST)
        .unwrap();
    assert_eq!(status, 405);
    assert_eq!(headers["allow"], "GET");
    for (path, method) in [
        ("/v1/auth/attempts", Method::POST),
        (
            "/auth/github/callback?code=synthetic&state=synthetic",
            Method::GET,
        ),
    ] {
        let (status, body, _) = sandbox.request(&base, path, method).unwrap();
        assert_eq!(status, 503);
        assert_eq!(body, json!({"error":"authentication_unavailable"}));
    }
    assert_eq!(
        sandbox.request(&base, "/unknown", Method::GET).unwrap().0,
        404
    );
    sandbox.stop();
    config["vars"]["AUTH_ORIGIN"] = json!("http://invalid.example.com");
    sandbox.configure(&config);
    let base = sandbox.start();
    sandbox.wait(&base, "/v1/auth/config", |status, body| {
        status == 503 && body == &json!({"error":"authentication_unavailable"})
    });
    sandbox.stop();
}

#[test]
#[ignore = "requires Docker; run explicitly with coverage in CI"]
fn fixture_wasm_reload_and_persistence() {
    use reqwest::Method;
    use serde_json::json;
    support::build("tools/celld/build.sh");
    let mut sandbox = support::Sandbox::new();
    let source = support::root().join("tools/celld/fixture");
    for file in ["worker.mjs", "build/probe.wasm"] {
        sandbox.copy(&source.join(file), file);
    }
    let config =
        serde_json::from_str(&std::fs::read_to_string(source.join("wrangler.jsonc")).unwrap())
            .unwrap();
    sandbox.configure(&config);
    let base = sandbox.start();
    sandbox.wait(&base, "/", |status, body| {
        status == 200 && body == &json!({"runtime":"celld","wasm":42})
    });
    for count in [1, 2] {
        assert_eq!(
            sandbox.request(&base, "/counter", Method::POST).unwrap().1,
            json!({"value":count})
        );
    }
    let module = sandbox.project.path().join("worker.mjs");
    let updated = std::fs::read_to_string(&module)
        .unwrap()
        .replace("probe.add(19, 23)", "probe.add(20, 23)");
    std::fs::write(module, updated).unwrap();
    sandbox.wait(&base, "/", |status, body| {
        status == 200 && body == &json!({"runtime":"celld","wasm":43})
    });
    assert_eq!(
        sandbox.request(&base, "/counter", Method::POST).unwrap().1,
        json!({"value":3})
    );
    sandbox.stop();
    let base = sandbox.start();
    sandbox.wait(&base, "/", |status, body| {
        status == 200 && body == &json!({"runtime":"celld","wasm":43})
    });
    assert_eq!(
        sandbox.request(&base, "/counter", Method::POST).unwrap().1,
        json!({"value":4})
    );
    sandbox.stop();
}

#[cfg(unix)]
#[test]
fn command_diagnostics_do_not_corrupt_machine_readable_output() {
    let output = support::run(
        std::process::Command::new("sh").args(["-c", "printf success; printf diagnostic >&2"]),
        std::time::Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(output, "success");
}

#[test]
#[ignore = "requires Docker; verifies cleanup while unwinding"]
fn failed_smoke_cleans_its_containers_and_temporary_data() {
    support::build("tools/celld/build.sh");
    let mut sandbox = support::Sandbox::new();
    let name = sandbox.name().to_owned();
    let directory = sandbox.project.path().to_owned();
    let source = support::root().join("tools/celld/fixture");
    for file in ["worker.mjs", "build/probe.wasm"] {
        sandbox.copy(&source.join(file), file);
    }
    let config =
        serde_json::from_str(&std::fs::read_to_string(source.join("wrangler.jsonc")).unwrap())
            .unwrap();
    sandbox.configure(&config);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let base = sandbox.start();
        sandbox.wait(&base, "/", |status, _| status == 200);
        panic!("synthetic verification failure");
    }));
    assert!(result.is_err());
    assert!(!directory.exists());
    let containers = support::run(
        std::process::Command::new("docker").args([
            "ps",
            "-aq",
            "--filter",
            &format!("label=com.docker.compose.project={name}"),
        ]),
        std::time::Duration::from_secs(10),
    )
    .unwrap();
    assert!(containers.is_empty());
}
