---
title: "Transformers and metadata"
description: "Add metadata to nodes with closures, the built-in LLM metadata transformers and their prompts, and your own transformers and batch transformers."
sidebar:
  order: 4
identLint:
  ignore: [include_str!]
---

You add metadata to nodes with closures, with the built-in transformers that ask a large language model (LLM), or with a transformer you write yourself. Metadata travels with each node to the embedder, so a retriever can match on it and an answer can cite it.

## Closures

The [ask-your-codebase](/showcases/ask-your-codebase/) showcase stores each file's path, relative to the codebase root, on every node.

::snippet{src="@swiftide/examples/ask-your-codebase/src/index.rs" anchor="path_metadata" title="examples/ask-your-codebase/src/index.rs"}

A closure from `TextNode` to `Result<TextNode>` is a transformer if it is `Clone`, `Send`, `Sync` and `'static`, so it cannot borrow local data. You add it with `.then(...)`. The pipeline runs it on many nodes at once and does not keep their order.

`node.metadata` is a `Metadata` map. `insert` takes a key that converts into a `String` and a value that converts into a `serde_json::Value`.

LanceDB and PgVector store a field only after you declare it with `with_metadata`, which is why the showcase calls `.with_metadata(PATH_FIELD)`.

## Built-in metadata transformers

Swiftide ships six transformers that prompt the LLM per node and store the reply as metadata:

| Transformer | Metadata field | Feature flag |
| --- | --- | --- |
| `MetadataQAText` | `Questions and Answers (text)` | none |
| `MetadataQACode` | `Questions and Answers (code)` | `tree-sitter` |
| `MetadataSummary` | `Summary` | none |
| `MetadataTitle` | `Title` | none |
| `MetadataKeywords` | `Keywords` | none |
| `CompressCodeOutline` | `Outline` | `tree-sitter` |

Each module exports the field name as a `NAME` constant, such as `metadata_title::NAME`. `CompressCodeOutline` skips nodes without an `Outline` field; [Code-aware indexing](/indexing/code-aware-indexing/) shows how to fill it. The next example needs the `openai` and `qdrant` feature flags.

::snippet{src="@swiftide/examples/index_markdown_lots_of_metadata.rs" anchor="pipeline" title="examples/index_markdown_lots_of_metadata.rs"}

`new(client)` uses the default prompt. `from_client(client)` returns the builder, where you set `prompt_template`, `concurrency` and, for the question transformers, `num_questions` (default 5). `builder()` and `Default` start without a client.

After the chunker, each chunk gets its own prompt and reply. Before it, one prompt per file is enough, and chunking copies the reply to every chunk of that file.

`concurrency(n)` on the builder, or `with_concurrency(n)`, limits the requests that step sends at once. [Concurrency](/indexing/streams-and-concurrency/#concurrency) explains how this interacts with the pipeline value.

By default the embedder sees every metadata field as a `key: value` line in front of the chunk. [Embed mode](/indexing/embedding/#embed-mode) explains how to embed fields separately.

## Prompts

The default prompts show the expected answer inside a code fence, and models tend to copy that fence into the metadata. The [docs-qa](/showcases/docs-qa/) showcase replaces the `MetadataTitle` and `MetadataKeywords` prompts:

::snippet{src="@swiftide/examples/docs-qa/src/index.rs" anchor="metadata_prompts" title="examples/docs-qa/src/index.rs"}

The pipeline passes them to the builders with `prompt_template`:

::snippet{src="@swiftide/examples/docs-qa/src/index.rs" anchor="pipeline" title="examples/docs-qa/src/index.rs" mark="7-16"}

`prompt_template` takes anything that converts into a `Prompt`, such as a `&'static str` holding a Tera template. Each transformer adds the node as `node`, so `{{ node.chunk }}` is the chunk and `{{ node.metadata.Page }}` is a field that the docs-qa loader set. The question transformers also add `{{ questions }}`, and `MetadataQACode` adds `{{ outline }}` when the node has an `Outline` field.

In the captured docs-qa run, four pages became 43 chunks, and the title and keywords steps made 86 calls to `claude-haiku-5-5` through an OpenAI-compatible gateway to Amazon Web Services (AWS) Bedrock. The whole run, including local embedding and the writes to Qdrant, took:

```text
Indexed 43 chunks into `docs-qa` in 71.7s
```

The ask-your-codebase prompt asks for questions a developer would ask:

::snippet{src="@swiftide/examples/ask-your-codebase/src/index.rs" anchor="questions_prompt" title="examples/ask-your-codebase/src/index.rs"}

See [Prompts and templates](/models/prompts-and-templates/) for Tera syntax.

## Default LLM client

You can set one client on the pipeline instead of on each transformer. This example needs the `openai` feature flag.

::snippet{src="@snippets/src/indexing/transformers.rs" anchor="default_llm_client"}

`then` hands the pipeline defaults to the transformer when you add the step, so call `with_default_llm_client` first. A client on the transformer takes precedence. Without either, each node fails with `No client provided`.

## Custom transformers

A transformer is a type that implements `Transformer`.

::snippet{src="@snippets/src/indexing/transformers.rs" anchor="custom_transformer"}

`transform_node` receives one node and returns it, changed, or an error. The type must be `Clone`, `Send`, `Sync` and `'static`. The trait is an `async_trait`, so add the `async-trait` crate. Override `concurrency` to give the step its own limit.

`then` also requires `WithIndexingDefaults`; an empty `impl` is enough.

### Transformer macro

The built-in LLM transformers use `#[indexing_transformer]`. This one asks who a chunk is for:

::snippet{src="@snippets/src/indexing/transformers.rs" anchor="macro_transformer"}

`metadata_field_name` becomes the `NAME` constant. `default_prompt_file` goes to `include_str!`, relative to the source file. The macro adds `client`, `prompt_template` and `concurrency` fields, and generates `builder()`, `new`, `from_client`, `with_concurrency`, `Default` and the `WithIndexingDefaults` impl. `derive(skip_debug, skip_clone, skip_default)` turns off generated derives.

You still implement `Transformer`. Build the prompt with `with_node(&node)` before you pass it to the generated `self.prompt(...)`, which uses the transformer's client or the pipeline default. Override `concurrency()` as the example does, or `with_concurrency` has no effect.

The macro adds a `hidden` module, `NAME` and `default_prompt()` next to the struct, so give each annotated struct its own module. `new` and `Default` unwrap the builder, so give each of your own fields `#[builder(default = ...)]`.

Your crate needs `swiftide-core`, `anyhow` and `derive_builder` as direct dependencies; [Crates](/reference/crates/) has the `cargo add` line.

## Batch transformers

A batch transformer takes a `Vec` of nodes and returns a stream of any length. This one drops repeated chunks:

::snippet{src="@snippets/src/indexing/transformers.rs" anchor="batch_transformer"}

You add it with `then_in_batch`:

::snippet{src="@snippets/src/indexing/transformers.rs" anchor="custom_steps"}

The pipeline groups nodes into batches of `batch_size()`, or 256 when it returns `None`. `into()` turns a `Vec<TextNode>` into an `IndexingStream`. `then_in_batch` requires `WithBatchIndexingDefaults`, again with an empty `impl`.

A closure from `Vec<TextNode>` to `IndexingStream<String>` is a batch transformer too, with the pipeline batch size. `Embed` and `SparseEmbed` are batch transformers.

Batches are formed in arrival order, so the example only catches repeats that land in the same batch.

## Node types

`Transformer` and `BatchableTransformer` declare `Input` and `Output` types, both bound by `Chunk`. `then` returns a pipeline of the transformer's `Output`, so a different `Output` changes the node type for every step after it. Closures and most built-in transformers, chunkers and storage backends take `String` nodes, so add a transformer that changes the node type after every step that needs `String`.

## Reference

- [`Pipeline::then`](docsrs:swiftide/indexing/struct.Pipeline.html#method.then)
- [`Pipeline::then_in_batch`](docsrs:swiftide/indexing/struct.Pipeline.html#method.then_in_batch)
- [`Pipeline::with_default_llm_client`](docsrs:swiftide/indexing/struct.Pipeline.html#method.with_default_llm_client)
- [`Pipeline::with_concurrency`](docsrs:swiftide/indexing/struct.Pipeline.html#method.with_concurrency)
- [`Transformer`](docsrs:swiftide/traits/trait.Transformer.html)
- [`BatchableTransformer`](docsrs:swiftide/traits/trait.BatchableTransformer.html)
- [`WithIndexingDefaults`](docsrs:swiftide/traits/trait.WithIndexingDefaults.html)
- [`WithBatchIndexingDefaults`](docsrs:swiftide/traits/trait.WithBatchIndexingDefaults.html)
- [`Node`](docsrs:swiftide/indexing/struct.Node.html) and [`TextNode`](docsrs:swiftide/indexing/type.TextNode.html)
- [`Chunk`](docsrs:swiftide/indexing/trait.Chunk.html)
- [`Metadata`](docsrs:swiftide/indexing/struct.Metadata.html)
- [`IndexingStream`](docsrs:swiftide/indexing/struct.IndexingStream.html)
- [`MetadataQAText`](docsrs:swiftide/indexing/transformers/struct.MetadataQAText.html)
- [`MetadataQACode`](docsrs:swiftide/indexing/transformers/struct.MetadataQACode.html)
- [`MetadataSummary`](docsrs:swiftide/indexing/transformers/struct.MetadataSummary.html)
- [`MetadataTitle`](docsrs:swiftide/indexing/transformers/struct.MetadataTitle.html)
- [`MetadataKeywords`](docsrs:swiftide/indexing/transformers/struct.MetadataKeywords.html)
- [`CompressCodeOutline`](docsrs:swiftide/indexing/transformers/struct.CompressCodeOutline.html)
- [`Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html) and [`SparseEmbed`](docsrs:swiftide/indexing/transformers/struct.SparseEmbed.html)
- [`indexing_transformer`](docsrs:swiftide/attr.indexing_transformer.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)

## Next

[Embedding](/indexing/embedding/)
