use worker::*;

use crate::{oidc, supabase};

/// A minimal Worker entrypoint used by issue #192 to verify that Rust code can
/// be built for the Workers Wasm runtime.
#[event(fetch)]
pub async fn fetch(request: Request, env: Env, _context: Context) -> Result<Response> {
    if request.path() == "/spike/oidc/start" {
        return oidc::authorization_probe();
    }
    if request.path() == "/spike/oidc/verify" {
        return oidc::id_token_probe();
    }
    if request.path() == "/spike/oidc/discovery" {
        return oidc::discovery_probe().await;
    }
    if request.path() == "/spike/supabase/start" {
        return supabase::authorization_probe(&env);
    }
    if request.path() == "/spike/supabase/jwks" {
        return supabase::jwks_probe(&env).await;
    }

    Response::ok("Time Wise Workers compatibility check")
}
