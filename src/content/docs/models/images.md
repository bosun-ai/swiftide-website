---
title: "Images and multimodal input"
description: "Send images, and on some providers documents, audio and video, to a chat model with ChatMessage::new_user_with_parts and content parts."
sidebar:
  order: 4
---

By the end of this page you send an image and a question to a chat model in one request and print its answer.

The example uses the OpenAI client, which sits behind the `openai` feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features openai
```

::snippet{src="@snippets/src/models/images.rs" anchor="image_request"}

## Content parts

`openai` is an `OpenAI` client built as in [Models](/models/overview/). The prompt model you set with `default_prompt_model` must accept images, for example `gpt-4o-mini`. A client without a model fails with `Model not set`.

`ChatMessage::new_user_with_parts` builds a `ChatMessage::UserWithParts` message. It holds a list of content parts (`ChatMessageContentPart`) instead of one string. Here the list has a text part and an image part. The client sends them in that order.

Swiftide 0.33 added `UserWithParts`. It is a breaking change: if you match on `ChatMessage`, you need an arm for the new variant.

Each media part points at its data through a source (`ChatMessageContentSource`). `ChatMessageContentSource::bytes` takes the raw file and an optional media type. The OpenAI client encodes the bytes as base64 and sends them as a data Uniform Resource Locator (URL), `data:image/png;base64,...`. You do not need a base64 crate yourself.

`complete` is the method of the `ChatCompletion` trait, so import `swiftide::traits::ChatCompletion`. `SimplePrompt` only takes a text prompt, so it cannot carry images.

## Image sources

A source can be a URL, a data URL, bytes, an Amazon Web Services (AWS) Simple Storage Service (S3) object or a provider file id:

::snippet{src="@snippets/src/models/images.rs" anchor="image_sources"}

- A `&str` or `String` becomes a URL source. That covers both `https://` URLs and data URLs.
- `ChatMessageContentSource::file_id` refers to a file you uploaded to the provider earlier.
- `ChatMessageContentSource::s3` points at an object in S3. The second argument is the bucket owner. Without an owner, write `None::<String>`, because a bare `None` leaves its type unknown and does not compile.
- A `Vec<u8>` or `&[u8]` becomes a bytes source without a media type.
- `ChatMessageContentPart::image_with_format` sets the format, such as `png`, explicitly. AWS Bedrock reads it first and otherwise infers the format from the media type or the file extension. The OpenAI clients ignore it.

## Other content parts

Besides text and images, a message can carry document, audio and video parts. The helpers are `ChatMessageContentPart::document`, `document_with_name`, `audio` and `video`. A document with a file name looks like this:

::snippet{src="@snippets/src/models/images.rs" anchor="document_message"}

AWS Bedrock sends the name as the document name. The OpenAI Responses application programming interface (API) sends it as the file name of a bytes document. Few providers accept these parts, so check the table below before you use them.

The `audio`, `document` and `video` helpers leave the format unset, and only images have a `_with_format` helper. To set a format on another part, build the variant directly:

::snippet{src="@snippets/src/models/images.rs" anchor="wav_audio"}

## Provider support

Every client converts content parts to its provider's format before it sends the request. A part or source the client cannot convert fails `complete` with `LanguageModelError::PermanentError`, before anything reaches the provider. `complete_stream` returns a stream instead, and the error is its first item.

| Client | Image | Document | Audio | Video |
| --- | --- | --- | --- | --- |
| OpenAI, Chat Completions (default) | URL, data URL, bytes | no | bytes, `mp3` or `wav` | no |
| OpenAI, Responses API (`use_responses_api(true)`) | URL, data URL, bytes, file id | URL, bytes, file id | no | no |
| AWS Bedrock (`aws-bedrock`) | bytes, base64 data URL, S3, `s3://` URL | same sources as image | same sources as image | same sources as image |
| Anthropic | no | no | no | no |

- Ollama, Groq, Gemini, OpenRouter and Dashscope are [OpenAI-compatible clients](/models/overview/#providers) and share the OpenAI conversion. Whether the endpoint accepts the image depends on the provider and the model you pick.
- On OpenAI Chat Completions, audio must be a bytes source in `mp3` or `wav`. Without a format it is sent as `mp3`, and any other format fails. The media type does not change this.
- AWS Bedrock accepts only base64 data URLs and `s3://` URLs. It rejects every other URL, such as `https://`, `http://` or `file://`, and it rejects file ids.
- AWS Bedrock needs a format for images, audio and video. Pass a media type, use a file extension in the URL, or set the format explicitly. Otherwise the call fails with `Bedrock image format is required`.
- An AWS Bedrock document without a resolvable format does not fail. It is sent as `txt`, so a PDF in bytes without a media type goes out as a text document.
- An AWS Bedrock document part needs a non-empty text part in the same message. Bedrock drops empty text parts, so `text("")` does not count.
- The Anthropic client (`anthropic` feature flag) accepts `UserWithParts` messages with text parts only. Any other part returns an error.

## Gotchas

- Token estimates ignore every part except text. See [Errors, backoff, usage and tokens](/models/errors-backoff-and-usage/).
- `ChatMessage::as_ref` returns an empty string for a `UserWithParts` message, unless the message has exactly one text part and nothing else. Code that logs or searches message text through `as_ref` skips these messages.
- The `Debug` output of a source cuts data URLs to their first 32 characters of data, and bytes sources show only their length. Debug logs of the message history stay short.
- For image parts without a media type, the OpenAI clients label bytes as `application/octet-stream`. Pass the real media type to `ChatMessageContentSource::bytes`. Documents on the Responses API and audio on Chat Completions do not send the media type, so there it has no effect.

## Reference

- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [`ChatMessageContentPart`](docsrs:swiftide/chat_completion/enum.ChatMessageContentPart.html)
- [`ChatMessageContentSource`](docsrs:swiftide/chat_completion/enum.ChatMessageContentSource.html)
- [`ChatCompletionRequest`](docsrs:swiftide/chat_completion/struct.ChatCompletionRequest.html)
- [`ChatCompletionResponse`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponse.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`LanguageModelError`](docsrs:swiftide/chat_completion/errors/enum.LanguageModelError.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)

## Next

[Errors, backoff, usage and tokens](/models/errors-backoff-and-usage/)
