//! Authentication Worker foundation, with provider traffic disabled.

use serde_json::json;
use url::Url;

#[derive(Debug, PartialEq)]
pub struct Configuration {
    auth_origin: String,
    client_profile_ids: Vec<String>,
}

impl Configuration {
    pub fn parse(origin: &str, profiles: &str) -> Result<Self, &'static str> {
        let invalid = "invalid configuration";
        let url = Url::parse(origin).map_err(|_| invalid)?;
        let canonical = url.origin().ascii_serialization();
        if origin != canonical && origin != format!("{canonical}/") {
            return Err(invalid);
        }
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(invalid);
        }
        let ids: Vec<String> = serde_json::from_str(profiles).map_err(|_| invalid)?;
        if ids.is_empty()
            || ids.len() > 16
            || ids.iter().enumerate().any(|(i, id)| {
                id.is_empty()
                    || id.len() > 64
                    || !id
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
                    || ids[..i].contains(id)
            })
        {
            return Err(invalid);
        }
        Ok(Self {
            auth_origin: url.origin().ascii_serialization(),
            client_profile_ids: ids,
        })
    }
}

pub fn respond(method: &str, path: &str, config: Option<&Configuration>) -> (u16, String) {
    if path == "/v1/auth/config" {
        if method != "GET" {
            return (405, json!({"error": "method_not_allowed"}).to_string());
        }
        if let Some(config) = config {
            return (200, json!({"protocol_version": 1, "auth_origin": config.auth_origin, "client_profile_ids": config.client_profile_ids}).to_string());
        }
    } else if !path.starts_with("/v1/auth/") && !path.starts_with("/auth/github/") {
        return (404, json!({"error": "not_found"}).to_string());
    }
    (
        503,
        json!({"error": "authentication_unavailable"}).to_string(),
    )
}

#[cfg(target_arch = "wasm32")]
#[worker::event(fetch)]
pub async fn fetch(
    req: worker::Request,
    env: worker::Env,
    _ctx: worker::Context,
) -> worker::Result<worker::Response> {
    let config = env
        .var("AUTH_ORIGIN")
        .ok()
        .zip(env.var("CLIENT_PROFILE_IDS").ok())
        .and_then(|(origin, profiles)| {
            Configuration::parse(&origin.to_string(), &profiles.to_string()).ok()
        });
    let (status, body) = respond(&req.method().to_string(), &req.path(), config.as_ref());
    let mut response = worker::Response::ok(body)?.with_status(status);
    response
        .headers_mut()
        .set("Content-Type", "application/json")?;
    response.headers_mut().set("Cache-Control", "no-store")?;
    if status == 405 {
        response.headers_mut().set("Allow", "GET")?;
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_an_operator_origin_and_fork_profile() {
        assert!(Configuration::parse("https://auth.example.com", r#"["fork-release-v1"]"#).is_ok());
    }

    #[test]
    fn rejects_invalid_origins_and_profiles() {
        for origin in [
            "",
            "http://localhost:9876",
            "https:example.com",
            "https://auth.example.com/path/..",
            "https://@auth.example.com",
            "https://user:password@example.com",
            "https://example.com/path",
            "https://example.com?query=1",
            "https://example.com#fragment",
        ] {
            assert!(Configuration::parse(origin, r#"["release"]"#).is_err());
        }
        for profiles in ["[]", "not-json", r#"["same","same"]"#, r#"["bad/profile"]"#] {
            assert!(Configuration::parse("https://auth.example.com", profiles).is_err());
        }
    }

    #[test]
    fn discovery_returns_only_the_public_protocol_configuration() {
        let config =
            Configuration::parse("https://auth.example.com", r#"["fork-release-v1"]"#).unwrap();
        let (status, body) = respond("GET", "/v1/auth/config", Some(&config));
        assert_eq!(status, 200);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&body).unwrap(),
            json!({"protocol_version": 1, "auth_origin": "https://auth.example.com", "client_profile_ids": ["fork-release-v1"]})
        );
    }

    #[test]
    fn invalid_configuration_fails_closed_and_authentication_stays_disabled() {
        assert_eq!(
            respond("GET", "/v1/auth/config", None),
            (503, r#"{"error":"authentication_unavailable"}"#.into())
        );
        assert_eq!(respond("POST", "/v1/auth/attempts", None).0, 503);
        assert_eq!(respond("GET", "/auth/github/callback", None).0, 503);
    }

    #[test]
    fn discovery_rejects_mutation_and_unknown_routes_are_not_found() {
        assert_eq!(respond("POST", "/v1/auth/config", None).0, 405);
        assert_eq!(respond("GET", "/unknown", None).0, 404);
    }
}
