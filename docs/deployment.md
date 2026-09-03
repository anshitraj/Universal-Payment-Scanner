# Deploying the playground (e.g. to Vercel)

The playground is a static, client-side app - the Rust core runs as WebAssembly in the browser.
There is no server in the request path, so **no backend is required** to deploy it or to scan a
real QR code with it. The optional Go API (`services/api-go/`) is a separate thing entirely: it
exists for teams that would rather call a server-side HTTP endpoint than embed the WASM/JS SDK
directly. Skip it unless you specifically want that.

## Vercel

`vercel.json` at the repo root is already configured:

```json
{
  "framework": "vite",
  "installCommand": "npm install",
  "buildCommand": "npm run build:vercel",
  "outputDirectory": "apps/playground/dist"
}
```

Point a Vercel project at this repository (root directory = repo root, not `apps/playground`) and
it will build and deploy with no other configuration. No environment variables are required.

### Why `build:vercel` instead of `build`

The full `npm run build` starts with `npm run build:wasm`, which invokes `wasm-pack` - it needs a
Rust toolchain, which Vercel's build image doesn't have. `npm run build:vercel` skips that step and
uses the wasm-pack output already committed at `packages/wasm/generated/` and
`packages/wasm/generated-node/` (see the comment in `.gitignore` - these two directories are
intentionally *not* ignored, unlike every other build output in this repo).

**This means: after any change under `crates/`, you must rebuild and commit the WASM output before
redeploying**, or the deployed playground will keep running the old core:

```bash
npm run build:wasm
git add packages/wasm/generated packages/wasm/generated-node
git commit -m "chore: rebuild wasm"
```

This is a deliberate shortcut to keep the Vercel build fast and dependency-free while the project
is young. The more correct long-term setup is a CI job that builds the WASM package and either
publishes it to npm (so `apps/playground`'s `package.json` depends on a real published version
instead of workspace-aliased source) or gives Vercel's build a Rust toolchain via a custom install
command - either removes the "don't forget to recommit the binary" footgun. Worth doing before this
goes fully public; not necessary to test today.

## Camera scanning in production

Browsers only grant camera access (`getUserMedia`) on HTTPS or `localhost`. Vercel serves HTTPS by
default, so the camera tab works there with no extra configuration. It will *not* work if you open
a plain `http://<your-lan-ip>:port` URL from a phone during local development - use the deployed
HTTPS URL, or a tool like `vite --host` behind a TLS-terminating tunnel, to test camera scanning
from a phone before deploying.
