# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

jitRegistry is a Rust service that acts as a container registry, building container images on-demand (just-in-time) when pulled via podman. It bridges directory structures containing buildah scripts or Dockerfiles with container runtimes. It is NOT fully OCI-compliant by design.

**Requires:** Linux kernel > 3.14, buildah installed and in PATH. Cannot run inside a container.

## Build & Run

```bash
# Build
cargo build --release

# Run (flags or env vars)
jitRegistry -d /path/to/containers -b 127.0.0.1 -B 7999
# Env vars: JITREGISTRY_DIR, JITREGISTRY_BIND_ADDR, JITREGISTRY_BIND_PORT

# Test with podman
podman run localhost:7999/images/foo
```

No automated test suite exists. Testing is manual via podman pulls.

## Architecture

Three modules in `src/`:

- **main.rs** — Actix-web 4 HTTP server with two endpoints:
  - `GET /v2/` — health check (OCI v2 handshake)
  - `GET /v2/{name}/manifests/latest` — triggers build and returns OCI ImageManifest JSON

- **clilib.rs** — CLI args via structopt (`Cli`, `Args`, `WA` structs). `WA` is the shared web app state passed to Actix handlers.

- **buildah.rs** (module `b`) — buildah command integration. Key flow: discover build definition → execute buildah → extract hash from output → read manifest from `{graphroot}/overlay-images/{hash}/manifest`.

**Request flow:** podman request → Actix handler → search container directory for buildah script (.sh) or Dockerfile/Containerfile → buildah builds image → read manifest from buildah graphroot → return JSON.

## Key Dependencies

- `actix-web` 4.0 (HTTP), `structopt` 0.3 (CLI), `oci-spec` 0.5 (manifest types), `walkdir` 2.3 (directory traversal)

## Known Issues

- Podman can pull manifest but cannot fully read it (active debugging area)
- Blob serving endpoint not yet implemented (commented out)
- No caching — every pull triggers a rebuild
