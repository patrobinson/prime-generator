#![feature(split_array)]
use serde::{Deserialize, Serialize};
use worker::*;
use sha2::{Digest, Sha256};
use rand::prelude::*;
use rand_chacha::ChaCha12Rng;
use hex::FromHex;

mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

#[derive(Serialize, Deserialize)]
struct DrandResponse {
    round: u64,
    randomness: String,
    signature: String,
    previous_signature: String,
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::new();

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        .get("/", |_, _| Response::ok("Hello from Workers!"))
        .get_async("/random", |_, _ctx| async move {
            let rand = random().await;
            if let Err(e) = rand {
                return Response::error(e.to_string(), 500)
            }

            Response::ok(rand.unwrap().to_string())
        })
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env).await
}

async fn random() -> std::result::Result<u64, std::string::String> {
    let mut hasher = Sha256::new();
    let response = reqwest::get("https://drand.cloudflare.com/public/latest").await;
    if let Err(e) = response {
        let err = format!("Invalid drand response {}", e);
        return Err(err)
    }
    
    let v: DrandResponse = serde_json::from_str(&response.unwrap().text().await.unwrap()).unwrap();
    let signature = v.signature;
    hasher.update(signature);
    let h = format!("{:X}", hasher.finalize());
    let buf = Vec::from_hex(h).unwrap();
    let (seed, _) = buf.split_array_ref::<32>();

    let mut rng = ChaCha12Rng::from_seed(*seed);
    Ok(rng.gen_range(0..100000000) as u64)
}