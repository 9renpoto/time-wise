//! Validated, non-destructive deployment of the authentication foundation.

use serde_json::Value;
use std::{fs, path::Path, process::Command};
use time_wise_server::Configuration;

const MIGRATIONS: &str = "apps/server/migrations";

fn validate(value: &Value, deployment: bool) -> Result<(), String> {
    let string = |value: &Value| {
        value
            .as_str()
            .map(str::to_owned)
            .ok_or("missing configuration".to_owned())
    };
    Configuration::parse(
        &string(&value["vars"]["AUTH_ORIGIN"])?,
        &string(&value["vars"]["CLIENT_PROFILE_IDS"])?,
    )
    .map_err(str::to_owned)?;
    let databases = value["d1_databases"]
        .as_array()
        .ok_or("missing D1 binding")?;
    if databases.len() != 1
        || databases[0]["binding"] != "AUTH_DB"
        || databases[0]["migrations_dir"] != MIGRATIONS
        || string(&databases[0]["database_name"])?.is_empty()
    {
        return Err("expected one AUTH_DB binding and the repository migration directory".into());
    }
    if deployment {
        let id = string(&databases[0]["database_id"])?;
        if id.len() != 36
            || id.bytes().enumerate().any(|(i, c)| {
                if [8, 13, 18, 23].contains(&i) {
                    c != b'-'
                } else {
                    !c.is_ascii_hexdigit()
                }
            })
            || id == "00000000-0000-0000-0000-000000000000"
        {
            return Err("provision D1 and set database_id before deployment".into());
        }
    }
    Ok(())
}

fn execute(
    has_migrations: bool,
    mut run: impl FnMut(&[&str]) -> Result<(), String>,
) -> Result<(), String> {
    if has_migrations {
        run(&["d1", "migrations", "apply", "AUTH_DB", "--remote"])?;
    }
    run(&["deploy"])
}

fn has_migrations(directory: &Path) -> Result<bool, String> {
    let mut found = false;
    for entry in fs::read_dir(directory).map_err(|_| "cannot read migration directory")? {
        let entry = entry.map_err(|_| "cannot read migration entry")?;
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "sql")
        {
            if !entry
                .file_type()
                .map_err(|_| "cannot inspect migration")?
                .is_file()
            {
                return Err("migration must be a regular SQL file".into());
            }
            found = true;
        }
    }
    Ok(found)
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let deployment = match args.as_slice() {
        [command] if command == "check" => false,
        [command] if command == "deploy" => true,
        _ => return Err("usage: deployment check|deploy (run from repository root)".into()),
    };
    let config: Value = serde_json::from_slice(
        &fs::read("wrangler.json").map_err(|_| "cannot read wrangler.json")?,
    )
    .map_err(|_| "wrangler.json must contain valid JSON")?;
    validate(&config, deployment)?;
    let migrations = has_migrations(Path::new(MIGRATIONS))?;
    if !deployment {
        println!("Deployment template valid; authentication remains disabled. D1 provisioning is checked on deploy.");
        return Ok(());
    }
    execute(migrations, |args| {
        let executable = Path::new("node_modules/wrangler/bin/wrangler.js")
            .canonicalize()
            .map_err(|_| "run npm ci before deployment")?;
        // Node executes the locked local CLI on Unix and Windows alike.
        let status = Command::new("node")
            .arg(executable)
            .args(args)
            .args(["--config", "wrangler.json"])
            .status()
            .map_err(|_| "could not start Wrangler")?;
        if status.success() {
            Ok(())
        } else {
            Err("Wrangler failed; deployment stopped".into())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn configuration() -> serde_json::Value {
        json!({
            "name": "synthetic-auth",
            "vars": {"AUTH_ORIGIN": "https://auth.example.com", "CLIENT_PROFILE_IDS": "[\"fork-v1\"]"},
            "d1_databases": [{"binding": "AUTH_DB", "database_name": "synthetic-auth", "database_id": "11111111-2222-3333-4444-555555555555", "migrations_dir": "apps/server/migrations"}]
        })
    }

    #[test]
    fn rejects_invalid_configuration_before_any_command() {
        for value in [
            {
                let mut v = configuration();
                v["vars"]["AUTH_ORIGIN"] = json!("http://example.com");
                v
            },
            {
                let mut v = configuration();
                v["vars"]["CLIENT_PROFILE_IDS"] = json!("[]");
                v
            },
            {
                let mut v = configuration();
                v["d1_databases"][0]["binding"] = json!("OTHER_DB");
                v
            },
            {
                let mut v = configuration();
                v["d1_databases"][0]["migrations_dir"] = json!("../outside");
                v
            },
        ] {
            assert!(validate(&value, true).is_err());
        }
    }

    #[test]
    fn template_can_be_checked_but_cannot_deploy_without_provisioned_database() {
        for id in ["", "00000000-0000-0000-0000-000000000000", "not-an-id"] {
            let mut v = configuration();
            v["d1_databases"][0]["database_id"] = json!(id);
            assert!(validate(&v, false).is_ok());
            assert!(validate(&v, true).is_err());
        }
    }

    #[test]
    fn repeated_deployment_uses_binding_migrations_then_deploy_without_reset() {
        let mut calls = Vec::new();
        for _ in 0..2 {
            execute(true, |args| {
                calls.push(args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>());
                Ok(())
            })
            .unwrap();
        }
        assert_eq!(
            calls,
            vec![
                vec!["d1", "migrations", "apply", "AUTH_DB", "--remote"],
                vec!["deploy"],
                vec!["d1", "migrations", "apply", "AUTH_DB", "--remote"],
                vec!["deploy"],
            ]
        );
    }

    #[test]
    fn migration_failure_prevents_worker_deployment() {
        let mut calls = Vec::new();
        let result = execute(true, |args| {
            calls.push(args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>());
            Err("migration failed".into())
        });
        assert!(result.is_err());
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0][0], "d1");
    }

    #[test]
    fn empty_migration_directory_deploys_without_inventing_schema() {
        let mut calls = Vec::new();
        execute(false, |args| {
            calls.push(args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>());
            Ok(())
        })
        .unwrap();
        assert_eq!(calls, vec![vec!["deploy"]]);
    }

    #[test]
    fn scans_only_regular_sql_files_and_fails_on_missing_directory() {
        let directory = tempfile::tempdir().unwrap();
        assert!(!has_migrations(directory.path()).unwrap());
        fs::write(directory.path().join("README.md"), "schema is deferred").unwrap();
        assert!(!has_migrations(directory.path()).unwrap());
        fs::write(directory.path().join("0001_synthetic.sql"), "SELECT 1;").unwrap();
        assert!(has_migrations(directory.path()).unwrap());
        fs::create_dir(directory.path().join("0002_invalid.sql")).unwrap();
        assert!(has_migrations(directory.path()).is_err());
        assert!(has_migrations(&directory.path().join("missing")).is_err());
    }
}
