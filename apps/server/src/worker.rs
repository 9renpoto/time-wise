use worker::*;

/// A minimal Worker entrypoint used by issue #192 to verify that Rust code can
/// be built for the Workers Wasm runtime.
#[event(fetch)]
pub async fn fetch(_request: Request, _env: Env, _context: Context) -> Result<Response> {
    Response::ok("Time Wise Workers compatibility check")
}
