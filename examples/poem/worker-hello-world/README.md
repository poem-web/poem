# Poem with Cloudflare worker

## Prerequisites

- [worker-build](https://github.com/cloudflare/workers-rs/tree/main/worker-build) 0.8, matching the `worker` crate's minor version
- [Wrangler](https://developers.cloudflare.com/workers/wrangler/install-and-update/)

Install the matching build tool with `cargo install worker-build --version '^0.8' --locked`.
Run `wrangler dev` in this directory to build and serve the example locally.
The build output is `build/index.js`.
