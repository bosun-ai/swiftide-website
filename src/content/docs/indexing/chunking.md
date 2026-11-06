---
title: "Chunking"
description: "Split source code with tree-sitter and markdown or plain text by characters, write a chunker as a closure, and keep path and metadata on every chunk."
sidebar:
  order: 3
---

After this page your indexing pipeline splits code along syntax boundaries, markdown and text by characters, and any other format with a closure, with chunks sized for your embedder. A chunker takes one node and returns many; you add it with `then_chunk`.

## Source code

`ChunkCode` needs the `tree-sitter` [feature flag](/reference/feature-flags/):

```sh
cargo add swiftide@{{swiftide_version}} --features tree-sitter
```

::snippet{src="@snippets/src/indexing/chunking.rs" anchor="code_chunkers"}

`ChunkCode` parses each file with tree-sitter and splits it along syntax boundaries such as functions, `impl` blocks and classes. It adds top-level items to a chunk. It closes the chunk when the next item would bring it to the maximum or past it. An item that is too large on its own is split along its children.

The size is a `ChunkSize`, and it counts bytes. A number such as `1500` is a maximum. A range such as `200..2000` adds a minimum: a chunk of 200 bytes or less is dropped. `try_for_language` uses a maximum of 1500 bytes. The language is a `SupportedLanguages` value or a name such as `"python"`; an unknown name returns an error.

`with_concurrency` sets how many files are chunked at once. Without it the chunker uses the pipeline's concurrency.

When tree-sitter finds a syntax error, `ChunkCode` logs a warning and returns the whole file as one chunk, whatever its size.

## Several languages

`ChunkCode` handles one language. The ask-your-codebase showcase indexes several in one pipeline with its own chunker:

::snippet{src="@swiftide/examples/ask-your-codebase/src/languages.rs" anchor="chunker" title="examples/ask-your-codebase/src/languages.rs"}

The showcase builds it with a range of `10..4096` bytes. The [tree-sitter integration](/integrations/tree-sitter/#several-languages-in-one-pipeline) explains how it picks a language when extensions overlap.

A file without a matching chunker becomes an error in the stream, and the first error stops `run`. Load only the extensions you have chunkers for, as the showcase does, or add `log_errors()` and `filter_errors()` ([Streams and concurrency](/indexing/streams-and-concurrency/)).

## Markdown and text

::snippet{src="@snippets/src/indexing/chunking.rs" anchor="text_chunkers"}

`ChunkMarkdown` and `ChunkText` use the text-splitter crate and count characters, not bytes. `ChunkMarkdown` prefers to split at headings, then at paragraphs and other blocks. `ChunkText` splits at blank lines and line breaks, then at sentences and words. Both default to a maximum of 2056 characters.

With `from_chunk_range`, text-splitter treats the start of the range as the size to aim for and the end, exclusive, as a hard limit. A chunk grows past the start only when that keeps a larger unit, such as a section, in one piece. Chunks that are empty after trimming, or shorter than the start, are dropped. That check uses the byte length of the trimmed text, which equals the character count only for ASCII.

The [docs-qa showcase](/showcases/docs-qa/) splits documentation pages with `ChunkMarkdown::from_chunk_range(120..1500)`, because shorter chunks were mostly badges and stray headings.

## Closures

::snippet{src="@snippets/src/indexing/chunking.rs" anchor="closure_chunker"}

A closure `Fn(TextNode) -> IndexingStream<String>` that is `Clone + Send + Sync` is a chunker. It works on text nodes only. `IndexingStream::iter` turns an iterator of `Result<TextNode>` into a stream.

`node.into_chunks` builds one node per chunk from the input node. Every chunk keeps the path, metadata, vectors and embed mode of that node. For anything more involved, or for other node types, implement `ChunkerTransformer` on a struct as the showcase does.

## Node expansion

::snippet{src="@snippets/src/indexing/chunking.rs" anchor="then_expand"}

`then_expand` runs the same `ChunkerTransformer` as `then_chunk`; only its tracing span is named differently. Use it when one node holds several records, as in this JSON Lines file, and use `then_chunk` when you cut content to size.

## Metadata and parents

Metadata that a node carries before chunking is copied to every chunk. The showcase stores each file's relative path as metadata before it chunks, so every chunk can cite its file. The same holds for metadata written by a large language model (LLM). Put a transformer such as `MetadataQACode` before the chunker and every chunk of a file carries the questions for the whole file. Put it after the chunker for metadata per chunk, at one request per chunk. `MetadataQACode` also needs the `tree-sitter` feature flag.

Since 0.33 every chunk also carries `parent_id`, the id its source node had when it reached the first chunker. The node cache marks that id when the file's chunks finish. Two consequences matter while you chunk:

- A step between `filter_cached` and the chunker that changes the path or the chunk breaks that match, so the file is never cached. Edit the content before `filter_cached`, as the showcase does.
- A file whose chunks all fall under the minimum is never cached and is processed again on every run.

[Cache keys](/indexing/caching/#cache-keys) explains the match in full.

## Gotchas

- Size chunks for the embedder. With the default embed mode the metadata is embedded together with the chunk, so leave room for it under the model's input limit.
- Chunks come out of `then_chunk` in no particular order, and chunks of different files interleave. `ChunkCode` sets `offset` on each chunk, a running byte count over the chunks before it in the same file. The text chunkers keep the offset of the input node.
- Only `ChunkCode` sees syntax. Running `ChunkText` over code cuts through functions at line or sentence boundaries.
- `ChunkCode` cannot split a syntax node that has no children. A single token larger than the maximum, such as a long comment or string, is silently left out of the chunks.

## Reference

- [`ChunkCode`](docsrs:swiftide/indexing/transformers/struct.ChunkCode.html)
- [`ChunkSize`](docsrs:swiftide/integrations/treesitter/enum.ChunkSize.html)
- [`SupportedLanguages`](docsrs:swiftide/integrations/treesitter/enum.SupportedLanguages.html)
- [`ChunkMarkdown`](docsrs:swiftide/indexing/transformers/struct.ChunkMarkdown.html)
- [`ChunkText`](docsrs:swiftide/indexing/transformers/struct.ChunkText.html)
- [`ChunkerTransformer`](docsrs:swiftide/traits/trait.ChunkerTransformer.html)
- [`Pipeline::then_chunk`](docsrs:swiftide/indexing/struct.Pipeline.html#method.then_chunk) and [`Pipeline::then_expand`](docsrs:swiftide/indexing/struct.Pipeline.html#method.then_expand)
- [`Pipeline::filter_errors`](docsrs:swiftide/indexing/struct.Pipeline.html#method.filter_errors)
- [`Node::into_chunks`](docsrs:swiftide/indexing/struct.Node.html#method.into_chunks)
- [`IndexingStream`](docsrs:swiftide/indexing/struct.IndexingStream.html)
- The [ask-your-codebase](/showcases/ask-your-codebase/) and [docs-qa](/showcases/docs-qa/) showcases, the [tree-sitter integration](/integrations/tree-sitter/) and [feature flags](/reference/feature-flags/)

## Next

[Transformers and metadata](/indexing/transformers-and-metadata/)
