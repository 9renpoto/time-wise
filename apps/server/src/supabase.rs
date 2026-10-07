use openidconnect::{core::CoreJsonWebKeySet, reqwest, JsonWebKeySetUrl, PkceCodeChallenge};
use worker::{Env, Response, Result, Url};

const SUPABASE_URL_BINDING: &str = "SUPABASE_URL";
const CALLBACK_URL_BINDING: &str = "AUTH_CALLBACK_URL";

/// Confirms that a Supabase Google authorization URL can be built with PKCE.
///
/// This endpoint deliberately does not redirect or reveal the verifier. A real
/// flow must persist the verifier and bind it to a single-use callback before
/// sending the browser to Supabase.
pub fn authorization_probe(env: &Env) -> Result<Response> {
    let base_url = match configured_supabase_url(env) {
        Ok(url) => url,
        Err(message) => return Response::error(message, 503),
    };
    let callback_url = match configured_https_url(env, CALLBACK_URL_BINDING) {
        Ok(url) => url,
        Err(message) => return Response::error(message, 503),
    };
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let mut authorization_url = base_url
        .join("auth/v1/authorize")
        .map_err(|error| worker::Error::RustError(error.to_string()))?;
    authorization_url
        .query_pairs_mut()
        .append_pair("provider", "google")
        .append_pair("scopes", "openid")
        .append_pair("redirect_to", callback_url.as_str())
        .append_pair("code_challenge", challenge.as_str())
        .append_pair("code_challenge_method", "s256");

    let query_has = |name: &str, expected: Option<&str>| {
        authorization_url.query_pairs().any(|(key, value)| {
            key == name && !value.is_empty() && expected.is_none_or(|expected| value == expected)
        })
    };
    let generated = authorization_url.scheme() == "https"
        && authorization_url.path() == "/auth/v1/authorize"
        && !verifier.secret().is_empty()
        && query_has("provider", Some("google"))
        && query_has("scopes", Some("openid"))
        && query_has("redirect_to", Some(callback_url.as_str()))
        && query_has("code_challenge", Some(challenge.as_str()))
        && challenge.method().as_str() == "S256"
        && query_has("code_challenge_method", Some("s256"));

    if generated {
        Response::ok("Supabase authorization probe passed")
    } else {
        Response::error("Supabase authorization probe failed", 500)
    }
}

/// Fetches and parses the public signing keys exposed by a configured Supabase
/// Auth project. No publishable key or authentication secret is required.
pub async fn jwks_probe(env: &Env) -> Result<Response> {
    let base_url = match configured_supabase_url(env) {
        Ok(url) => url,
        Err(message) => return Response::error(message, 503),
    };
    let jwks_url = base_url
        .join("auth/v1/.well-known/jwks.json")
        .map_err(|error| worker::Error::RustError(error.to_string()))?;
    let jwks_url = JsonWebKeySetUrl::new(jwks_url.to_string())
        .map_err(|error| worker::Error::RustError(error.to_string()))?;
    let http_client = reqwest::ClientBuilder::new()
        .build()
        .map_err(|error| worker::Error::RustError(error.to_string()))?;
    let jwks = CoreJsonWebKeySet::fetch_async(&jwks_url, &http_client)
        .await
        .map_err(|error| worker::Error::RustError(error.to_string()))?;

    if jwks.keys().is_empty() {
        Response::error("Supabase JWKS probe found no signing keys", 502)
    } else {
        Response::ok("Supabase JWKS probe passed")
    }
}

fn configured_supabase_url(env: &Env) -> std::result::Result<Url, &'static str> {
    let url = configured_https_url(env, SUPABASE_URL_BINDING)?;
    if url.path() == "/" {
        Ok(url)
    } else {
        Err("Supabase project URL must not contain a path")
    }
}

fn configured_https_url(env: &Env, binding: &str) -> std::result::Result<Url, &'static str> {
    let value = env
        .var(binding)
        .map_err(|_| "Supabase probe configuration is incomplete")?;
    let url = Url::parse(&value.to_string())
        .map_err(|_| "Supabase probe configuration contains an invalid URL")?;
    let valid = url.scheme() == "https"
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none();

    if valid {
        Ok(url)
    } else {
        Err("Supabase probe configuration requires a plain HTTPS URL")
    }
}
