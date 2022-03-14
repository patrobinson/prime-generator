#![feature(split_array)]
use serde::{Deserialize, Serialize};
use worker::*;
use sha2::{Digest, Sha256};
use rand::prelude::*;
use rand_chacha::ChaCha12Rng;
use hex::FromHex;
use num_bigint::{ToBigUint, RandBigInt, BigUint};
use num_traits::{One, Zero};
use std::iter::repeat_with;

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
        // Generates a 2048bit random number
        .get_async("/random", |_, _ctx| async move {
            let rand = random().await;
            if let Err(e) = rand {
                return Response::error(e.to_string(), 500)
            }

            Response::ok(rand.unwrap().to_string())
        })
        .get_async("/prime", |_, _ctx| async move {
            let rand = random().await;
            if let Err(e) = rand {
                return Response::error(e.to_string(), 500)
            }

            let p = find_next_prime(rand.unwrap());

            Response::ok(p.to_string())
        })
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env).await
}

async fn random() -> std::result::Result<num_bigint::BigUint, std::string::String> {
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
    Ok(rng.gen_biguint(2048))
}

fn find_next_prime(n: num_bigint::BigUint) -> num_bigint::BigUint {
    let mut n = match is_odd(&n) {
        true => n,
        false => n + 1 as u8
    };
    while !is_prime(&n, 1) {
        n = n + num_bigint::BigUint::from(2 as u8)
    };
    n
}

fn is_odd(n: &num_bigint::BigUint) -> bool {
    !(n % num_bigint::BigUint::from(2 as u8) == num_bigint::BigUint::from(0 as u8))
}

// This is copied from https://github.com/cjayross/miller_rabin

/*
MIT License

Copyright (c) 2020 Calvin Jay Ross

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

macro_rules! biguint {
    ($e:expr) => {
        ($e).to_biguint().unwrap()
    };
}

fn decompose(n: &BigUint) -> (BigUint, BigUint) {
    let one = One::one();
    let ref two = biguint!(2);
    let mut d: BigUint = (n - 1u8).clone();
    let mut r: BigUint = Zero::zero();

    while &d % two == one {
        d /= two;
        r += 1u8;
    }

    (d, r)
}

fn miller_rabin(a: &BigUint, n: &BigUint, d: &BigUint, r: &BigUint) -> bool {
    let n_minus_one: BigUint = n - 1u8;
    let mut x = a.modpow(d, n);
    let mut count: BigUint = One::one();
    let ref two = biguint!(2);

    if x == One::one() || x == n_minus_one {
        return false;
    }

    while &count < r {
        x = x.modpow(two, n);

        if x == n_minus_one {
            return false;
        }

        count += 1u8;
    }

    true
}

pub fn is_prime<T: ToBigUint>(n: &T, k: usize) -> bool {
    let ref n = biguint!(n);
    let n_minus_one: BigUint = n - 1u8;
    let (ref d, ref r) = decompose(n);

    if n <= &One::one() {
        return false;
    } else if n <= &biguint!(3) {
        return true;
    } else if n <= &biguint!(0xFFFF_FFFF_FFFF_FFFFu64) {
        let samples: Vec<u8> = vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
        return samples
            .iter()
            .filter(|&&m| biguint!(m) < n_minus_one)
            .find(|&&a| miller_rabin(&biguint!(a), n, d, r))
            .is_none();
    }

    let mut rng = rand::thread_rng();
    let samples: Vec<BigUint> = repeat_with(|| rng.gen_biguint(n_minus_one.bits()))
        .filter(|m| m < &n_minus_one)
        .take(k)
        .collect();

    samples
        .iter()
        .find(|&a| miller_rabin(a, n, d, r))
        .is_none()
}
