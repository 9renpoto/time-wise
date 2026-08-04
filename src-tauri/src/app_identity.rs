//! Resolves observed Windows processes to product-level application identities.

use std::path::Path;

use crate::platform::ProcessIdentity;
use crate::usage_history::AppMetadata;

/// Converts the identity evidence collected by the platform adapter into a
/// stable product key and display metadata.
#[must_use]
pub fn resolve(process: &ProcessIdentity) -> AppMetadata {
    let executable = process.executable.display().to_string();
    let display_name = process
        .product_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| package_display_name(process))
        .map(str::to_owned)
        .unwrap_or_else(|| executable_display_name(&process.executable));

    let stable_key = if let Some(package_family) = non_empty(process.package_family_name.as_deref())
    {
        format!("windows-package:{}", normalize_key_part(package_family))
    } else if let Some(product_name) = non_empty(process.product_name.as_deref()) {
        let publisher = non_empty(process.company_name.as_deref()).unwrap_or("unknown-publisher");
        format!(
            "windows-product:{}:{}",
            normalize_key_part(publisher),
            normalize_key_part(product_name)
        )
    } else {
        format!("windows-executable:{}", normalize_path(&executable))
    };

    AppMetadata {
        stable_key,
        display_name,
        executable: Some(executable.clone()),
        // The dashboard can ask Windows Shell for the representative icon from
        // this source without persisting a window title or other user content.
        icon_source: Some(executable),
        icon_png: process.icon_png.clone(),
    }
}

fn package_display_name(process: &ProcessIdentity) -> Option<&str> {
    non_empty(process.application_user_model_id.as_deref())
        .and_then(|value| value.rsplit(['!', '.']).next())
        .filter(|value| !value.trim().is_empty())
}

fn executable_display_name(path: &Path) -> String {
    let path = path.to_string_lossy();
    let file_name = path.rsplit(['\\', '/']).next().unwrap_or_default();
    Path::new(file_name)
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Unknown application")
        .to_string()
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}

fn normalize_key_part(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn normalize_path(value: &str) -> String {
    value.trim().replace('/', "\\").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn process(executable: &str) -> ProcessIdentity {
        ProcessIdentity {
            process_id: 42,
            executable: PathBuf::from(executable),
            ..ProcessIdentity::default()
        }
    }

    #[test]
    fn package_family_is_stable_across_processes_and_profiles() {
        let mut browser = process(r"C:\Program Files\WindowsApps\Browser\browser.exe");
        browser.package_family_name = Some("Example.Browser_123abc".into());
        browser.application_user_model_id = Some("Example.Browser_123abc!Browser".into());

        let mut helper = process(r"C:\Program Files\WindowsApps\Browser\helper.exe");
        helper.package_family_name = browser.package_family_name.clone();
        helper.application_user_model_id = browser.application_user_model_id.clone();

        assert_eq!(resolve(&browser).stable_key, resolve(&helper).stable_key);
        assert_eq!(resolve(&browser).display_name, "Browser");
    }

    #[test]
    fn desktop_product_metadata_combines_multiple_executables() {
        let mut editor = process(r"C:\Apps\Editor\editor.exe");
        editor.company_name = Some("Example Corp.".into());
        editor.product_name = Some("Example Editor".into());

        let mut renderer = process(r"C:\Apps\Editor\renderer.exe");
        renderer.company_name = editor.company_name.clone();
        renderer.product_name = editor.product_name.clone();

        assert_eq!(resolve(&editor).stable_key, resolve(&renderer).stable_key);
        assert_eq!(resolve(&editor).display_name, "Example Editor");
    }

    #[test]
    fn executable_path_is_a_deterministic_fallback() {
        let upper = process(r"C:\Apps\Terminal.exe");
        let lower = process(r"c:\apps\terminal.exe");

        assert_eq!(resolve(&upper).stable_key, resolve(&lower).stable_key);
        assert_eq!(resolve(&upper).display_name, "Terminal");
        assert_eq!(
            resolve(&process("/opt/apps/Terminal.exe")).display_name,
            "Terminal"
        );
        assert_eq!(
            resolve(&upper).icon_source.as_deref(),
            Some(r"C:\Apps\Terminal.exe")
        );
    }
}
