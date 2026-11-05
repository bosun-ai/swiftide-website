---
title: "Describing images"
description: "Send a Portable Network Graphics (PNG) image to an OpenAI model as a base64 data URL in a multi-part user message and print a one-sentence description."
sidebar:
  order: 5
identLint:
  ignore: [include_bytes]
---

This recipe sends the Swiftide logo to `gpt-4o-mini` and prints the one-sentence description the model writes back. It is the `describe_image.rs` example from the swiftide repository: one user message that carries a text part and an image part, sent with a single chat completion call.

## Setup

To run the example, clone the swiftide repository, set `OPENAI_API_KEY` and run it from the repository root:

```sh
git clone https://github.com/bosun-ai/swiftide && cd swiftide
export OPENAI_API_KEY=sk-...
cargo run -p swiftide-examples --example describe-image
```

The examples crate turns on every integration, so the first build compiles all of them. It needs `cmake` and the Protocol Buffers compiler (`protoc`) on your machine.

To use the code in your own crate, add the OpenAI client, which sits behind the `openai` feature flag, and the `base64` crate to encode the image:

```sh
cargo add swiftide@{{swiftide_version}} --features openai
cargo add base64@0.22 anyhow
cargo add tokio --features macros,rt-multi-thread
```

## Client

::snippet{src="@swiftide/examples/describe_image.rs" anchor="setup"}

The prompt model must accept images. `gpt-4o-mini` does. The client reads the API key from `OPENAI_API_KEY`.

## Image data

::snippet{src="@swiftide/examples/describe_image.rs" anchor="image"}

`env!("CARGO_MANIFEST_DIR")` is the directory of the examples crate, fixed at compile time. The path climbs one level to the repository root and reads `images/logo.png`. The example then encodes the bytes as base64 and wraps them in a data Uniform Resource Locator (URL) of the form `data:image/png;base64,...`.

The snippets leave out the imports. `encode` comes from the `base64::Engine` trait, and `with_context` from `anyhow::Context`, so bring both into scope with `use base64::{Engine as _, engine::general_purpose};` and `use anyhow::Context as _;`. The request types come from `swiftide::chat_completion`, and `complete` from `swiftide::traits::ChatCompletion`.

The media type in the data URL tells the model how to decode the bytes. Use the type of the file you actually send, such as `image/jpeg` for a Joint Photographic Experts Group (JPEG) file.

## Request

::snippet{src="@swiftide/examples/describe_image.rs" anchor="request"}

The message carries a text part and an image part, in that order. The data URL is a string, so `ChatMessageContentPart::image` takes it as a URL source. [Images and multimodal input](/models/images/#content-parts) explains content parts and sources.

The request has no system message here. Add one with `ChatMessage::new_system` in front of the user message when you want to set the tone or length of the answer.

## Response

::snippet{src="@swiftide/examples/describe_image.rs" anchor="run"}

`complete` sends one request and returns a `ChatCompletionResponse`. `message()` returns the text of the answer as an `Option<&str>`.

## Variations

### Bytes instead of a data URL

You can skip the base64 step. Pass the raw bytes and the media type, as `ChatMessageContentSource::bytes(image_bytes, Some("image/png"))`, and the OpenAI client builds the same data URL for you. A plain `Vec<u8>` also works, but it has no media type, and the OpenAI clients then label the image `application/octet-stream`. [Images and multimodal input](/models/images/) shows this form in full.

### Images on the web

With the OpenAI client, an `https://` URL works as is: `ChatMessageContentPart::image("https://example.com/chart.png")`. The client passes the URL to the provider unchanged.

### Several images

A message can carry more than one image part. Add each one to the list, and refer to them by position in the text part, for example "Compare the first and the second image".

### Other providers

The Anthropic client rejects image parts, and Amazon Web Services (AWS) Bedrock needs an image format and accepts only `data:` and `s3://` URLs. The [provider support table](/models/images/#provider-support) lists what each client accepts.

## Gotchas

- `CARGO_MANIFEST_DIR` is a compile-time path. A binary you copy to another machine does not find the file there. Read the path from configuration or a command-line argument, or embed the file with `include_bytes!`.
- A large image makes a large request. Resize it before you encode it when the model does not need full resolution.

## Reference

- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [`ChatMessageContentPart`](docsrs:swiftide/chat_completion/enum.ChatMessageContentPart.html)
- [`ChatMessageContentSource`](docsrs:swiftide/chat_completion/enum.ChatMessageContentSource.html)
- [`ChatCompletionRequest`](docsrs:swiftide/chat_completion/struct.ChatCompletionRequest.html)
- [`ChatCompletionResponse`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponse.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)

## Next

[Usage metrics](/cookbook/usage-metrics/)
