# `fastnear-openapi-generator`

Small shared Rust helpers for generating checked-in OpenAPI YAML from typed FastNear service DTOs.

The crate stays intentionally small. It exists to support the repo-owned OpenAPI workflow used by
the FastNEAR API services:

`Rust DTOs -> Rust operation registry -> checked-in aggregate openapi/openapi.yaml -> mike-docs split + sync -> builder-docs direct docs runtime`

## Intended Consumers

- `transfers-api`
- `explorer-api`
- `kv-fastdata-server`
- `fastnear-api-server-rs`
- `neardata-server`

## Current Public Helper Surface

- collect component schemas from `schemars`
- build aggregate OpenAPI documents from Rust-defined operation metadata
- carry stable per-operation slug/title metadata so `mike-docs` can split aggregate specs into portal routes
- deep-merge generated docs with YAML overlays
- write deterministic YAML for checked-in `openapi/` artifacts
- support `--check` stale-spec verification

## Current Role In The Docs Stack

This crate is part of the generation side of the docs system, not the presentation side.

- service repos own request/response DTOs and operation registration
- `fastnear-openapi-generator` builds and checks aggregate `openapi/openapi.yaml`
- `mike-docs` syncs and splits those aggregate specs into docs-owned leaf files
- `builder-docs` renders the public docs experience from generated page models

## Ownership Boundaries

This crate does not own service-level API descriptions. Each service repo still owns:

- request and response DTOs
- operation registration
- any repo-local raw-schema helpers that are genuinely needed for dynamic payloads
- checked-in generated `openapi/` output

Only clearly shared, stable helpers should be added here.
