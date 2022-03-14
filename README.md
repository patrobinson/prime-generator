# Getting Started

An API for producing Prime numbers

## Quick Start

In one window `wrangler dev`, then run:
```
$ curl http://127.0.0.1:8787/prime
12421470606451032515201726749122029966273675768939641901540340457280749639703635750226674912339326203058040780967377506773970820045484808229265389394962121
```

## Usage

Use Wrangler to deploy to Cloudflare Workers

With `wrangler`, you can build, test, and deploy your Worker with the following commands: 

```bash
# compiles your project to WebAssembly and will warn of any issues
wrangler build 

# run your Worker in an ideal development workflow (with a local server, file watcher & more)
wrangler dev

# deploy your Worker globally to the Cloudflare network (update your wrangler.toml file for configuration)
wrangler publish
```

Read the latest `worker` crate documentation here: https://docs.rs/worker
