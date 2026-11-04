---
title: "Feature flags"
description: "Every feature flag of the swiftide crate, generated from its Cargo.toml."
sidebar:
  order: 1
---

Swiftide keeps its dependencies opt-in: each integration and each building block beyond the core pipelines sits behind a Cargo feature flag. This page is generated from `swiftide/Cargo.toml` at the pinned commit, so it always matches the version these docs describe.

```sh
cargo add swiftide@{{swiftide_version}} --features openai,qdrant
```

::feature-flags
