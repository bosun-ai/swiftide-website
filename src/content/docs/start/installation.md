---
title: "Installation"
description: "Add swiftide with the right feature flags, set up tokio and API keys, and run your first program."
sidebar:
  order: 2
---

Coming soon.

```sh
cargo add swiftide@{{swiftide_version}} --features openai
```

Create an [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html) client:

::snippet{src="@snippets/src/start/installation.rs" anchor="openai_client" title="src/main.rs" collapse="1-2"}
