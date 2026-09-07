use openidconnect::{
    core::{
        CoreAuthenticationFlow, CoreClient, CoreIdToken, CoreIdTokenVerifier, CoreJsonWebKey,
        CoreJsonWebKeySet, CoreProviderMetadata,
    },
    reqwest, AuthUrl, ClientId, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge, RedirectUrl, Scope,
    TokenUrl,
};
use std::str::FromStr;
use worker::{Error, Response, Result};

const GOOGLE_AUTHORIZATION_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_ISSUER: &str = "https://accounts.google.com";
const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const SPIKE_REDIRECT_URI: &str = "https://example.com/auth/callback";
const FIXTURE_ISSUER: &str = "https://issuer.example";
const FIXTURE_JWK: &str = concat!(
    r#"{
    "kty":"RSA",
    "kid":"spike-key",
    "use":"sig",
    "alg":"RS256",
    "n":"uy29QYRknb9VzQbJ8gBcrEZCf7jo2a5HfFLbuc193aoLs58KhYJeD-b"#,
    "aKSBzw3k2MRonrTDd5bOy8KYKuWJM_F2ZhWo6X6GxGNMS9QZ-U85IK5Pd5hX7JclFsopdLDbDfEzJ6iCwKonE16Hdtw8Hn0_Gd6sTSF8Qt5uVoyq2-MCX5tu07YDDKdE9IGbJuMjZoo7qZsq3Tk4fA2XvKx6jPd2q9kSaQDjI",
    r#"zNKCOFbJuj4uGFJNGn_oitGN9L9Sr8Rehiy0gJD8IYHKt7o4FrpPLHPQcwiCYHXs7BsyjmwQaDczgik4MCQf_Z6zw4UmKFnE7u0_9bbh1VYuL5MMO7gK7w",
    "e":"AQAB"
}"#
);
const FIXTURE_ID_TOKEN_HEADER: &str =
    "eyJhbGciOiJSUzI1NiIsImtpZCI6InNwaWtlLWtleSIsInR5cCI6IkpXVCJ9";
const FIXTURE_ID_TOKEN_CLAIMS: &str = "eyJpc3MiOiJodHRwczovL2lzc3Vlci5leGFtcGxlIiwiYXVkIjoidGltZS13aXNlLXNwaWtlIiwic3ViIjoiZml4dHVyZS11c2VyIiwiZXhwIjo0MTAyNDQ0ODAwLCJpYXQiOjE3MDAwMDAwMDAsIm5vbmNlIjoiZml4dHVyZS1ub25jZSJ9";
const FIXTURE_ID_TOKEN_SIGNATURE: &str = concat!(
    "qlWUCZw9i-R1J-PEnd1kRLTOpwOQE09haRRe7CJLlV_PhdPIj8hffij4WuuC8sNUsa39u29O2jhy1Fjcqb0UJGqaPBkCCBnro37xi_jYb_UshK3wdKxiTGGFJd3vkgnlRzIGKhFt4MgrtvomaQEIR5l9_3uAE84W1zBLxLd7BidWv0Wf1lYAhN3YnCLc3nQySkBTJTQeBbM4vqgFQ3S3prlganVSMMeqtoeMWpMlhcdBPoBb2OBYnfBuI0X_sJi132ee6U",
    "ePJuZvf7QyvbvD5yS6wjN61abA5cNm_d_4A2i-DJe5p_75CjzOhqs-sF3sQPT_RCgoDoBehxpLelx4qw"
);

/// Exercises the OIDC authorization primitives that previously failed at
/// runtime for the evaluated WebAuthn libraries.
pub fn authorization_probe() -> Result<Response> {
    let client = CoreClient::new(
        ClientId::new("time-wise-spike".to_owned()),
        parse_issuer_url(GOOGLE_ISSUER)?,
        CoreJsonWebKeySet::new(Vec::new()),
    )
    .set_auth_uri(parse_auth_url(GOOGLE_AUTHORIZATION_ENDPOINT)?)
    .set_token_uri(parse_token_url(GOOGLE_TOKEN_ENDPOINT)?)
    .set_redirect_uri(parse_redirect_url(SPIKE_REDIRECT_URI)?);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (authorization_url, state, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_owned()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let query_has = |name: &str| {
        authorization_url
            .query_pairs()
            .any(|(key, value)| key == name && !value.is_empty())
    };
    let generated = !state.secret().is_empty()
        && !nonce.secret().is_empty()
        && !pkce_verifier.secret().is_empty()
        && query_has("state")
        && query_has("nonce")
        && query_has("code_challenge")
        && query_has("code_challenge_method");

    if generated {
        Response::ok("OIDC authorization probe passed")
    } else {
        Response::error("OIDC authorization probe failed", 500)
    }
}

/// Verifies a fixed RS256 ID token and confirms that a modified signature is
/// rejected using the same code that would validate Google or Supabase JWTs.
pub fn id_token_probe() -> Result<Response> {
    let key: CoreJsonWebKey =
        serde_json::from_str(FIXTURE_JWK).map_err(|error| Error::RustError(error.to_string()))?;
    let verifier = CoreIdTokenVerifier::new_public_client(
        ClientId::new("time-wise-spike".to_owned()),
        parse_issuer_url(FIXTURE_ISSUER)?,
        CoreJsonWebKeySet::new(vec![key]),
    );
    let nonce = Nonce::new("fixture-nonce".to_owned());
    let fixture_id_token =
        format!("{FIXTURE_ID_TOKEN_HEADER}.{FIXTURE_ID_TOKEN_CLAIMS}.{FIXTURE_ID_TOKEN_SIGNATURE}");
    let valid_token = CoreIdToken::from_str(&fixture_id_token)
        .map_err(|error| Error::RustError(error.to_string()))?;
    let claims = valid_token
        .claims(&verifier, &nonce)
        .map_err(|error| Error::RustError(error.to_string()))?;

    let mut tampered_value = fixture_id_token;
    let last = tampered_value.pop().ok_or_else(|| {
        Error::RustError("the ID token fixture unexpectedly has no signature".to_owned())
    })?;
    tampered_value.push(if last == 'A' { 'B' } else { 'A' });
    let tampered_token = CoreIdToken::from_str(&tampered_value)
        .map_err(|error| Error::RustError(error.to_string()))?;
    let tampered_rejected = tampered_token.claims(&verifier, &nonce).is_err();

    if claims.subject().as_str() == "fixture-user" && tampered_rejected {
        Response::ok("OIDC ID token probe passed")
    } else {
        Response::error("OIDC ID token probe failed", 500)
    }
}

/// Retrieves Google's OIDC discovery document through the Wasm-compatible
/// HTTP client used for provider metadata, JWKS, and token endpoint requests.
pub async fn discovery_probe() -> Result<Response> {
    // The Wasm reqwest backend does not expose redirect controls. This probe
    // uses a fixed issuer and validates the returned issuer and endpoints;
    // production code needs a Worker Fetch adapter that rejects redirects.
    let http_client = reqwest::ClientBuilder::new()
        .build()
        .map_err(|error| Error::RustError(error.to_string()))?;
    let metadata =
        CoreProviderMetadata::discover_async(parse_issuer_url(GOOGLE_ISSUER)?, &http_client)
            .await
            .map_err(|error| Error::RustError(error.to_string()))?;

    let has_expected_metadata = metadata.issuer().as_str() == GOOGLE_ISSUER
        && metadata.authorization_endpoint().as_str() == GOOGLE_AUTHORIZATION_ENDPOINT
        && metadata
            .token_endpoint()
            .is_some_and(|endpoint| endpoint.as_str() == GOOGLE_TOKEN_ENDPOINT)
        && metadata.jwks_uri().url().scheme() == "https";

    if has_expected_metadata {
        Response::ok("OIDC discovery probe passed")
    } else {
        Response::error("OIDC discovery probe failed", 500)
    }
}

fn parse_auth_url(value: &str) -> Result<AuthUrl> {
    AuthUrl::new(value.to_owned()).map_err(|error| Error::RustError(error.to_string()))
}

fn parse_issuer_url(value: &str) -> Result<IssuerUrl> {
    IssuerUrl::new(value.to_owned()).map_err(|error| Error::RustError(error.to_string()))
}

fn parse_token_url(value: &str) -> Result<TokenUrl> {
    TokenUrl::new(value.to_owned()).map_err(|error| Error::RustError(error.to_string()))
}

fn parse_redirect_url(value: &str) -> Result<RedirectUrl> {
    RedirectUrl::new(value.to_owned()).map_err(|error| Error::RustError(error.to_string()))
}
