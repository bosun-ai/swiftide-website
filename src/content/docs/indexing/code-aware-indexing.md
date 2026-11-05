---
title: "Code-aware indexing"
description: "Index a codebase with tree-sitter so each code chunk carries a compressed outline of its file, questions about the code, and the symbols it defines and uses."
sidebar:
  order: 9
---

This page builds an indexing pipeline that splits source code along syntax boundaries and gives every chunk the context of its file: an outline trimmed to what the chunk uses, questions the code answers, and the symbols it defines and uses. Tree-sitter is a parser library with one grammar per language. Swiftide wraps it in a chunker and four transformers behind the `tree-sitter` feature flag. The example below also uses OpenAI, a Redis node cache and Qdrant storage:

```sh
cargo add swiftide@{{swiftide_version}} --features tree-sitter,openai,redis,qdrant
```

## Reduced context pipeline

This example indexes a Rust codebase. It sets up an OpenAI client, the Redis URL for the node cache and a chunk size. It needs `OPENAI_API_KEY`, plus Redis and Qdrant running locally:

::snippet{src="@swiftide/examples/index_codebase_reduced_context.rs" anchor="setup"}

The pipeline gives every chunk an outline of its file, trimmed to what that chunk uses:

::snippet{src="@swiftide/examples/index_codebase_reduced_context.rs" anchor="pipeline"}

The steps run in this order:

1. `filter_cached` skips files Redis has already seen, keyed on path and content as [Cache keys](/indexing/caching/#cache-keys) explains.
2. `OutlineCodeTreeSitter` parses the whole file and stores an outline in the `Outline` metadata field. An outline is the file with the bodies of functions and methods removed. Signatures, types, imports and comments stay. The second argument skips files smaller than that many bytes. Here it equals the chunk size, because a file that fits in one chunk needs no outline.
3. `MetadataQACode` asks a large language model (LLM) for questions and answers about the file. Its default prompt includes the outline when the node has one.
4. `ChunkCode` splits the file into chunks of more than 10 and up to 2048 bytes.
5. `CompressCodeOutline` asks the LLM to keep only the outline lines the chunk needs and writes the result back to `Outline`.
6. `Embed` embeds the chunks in batches of 10, and Qdrant stores them with vectors of 1536 dimensions, the size of `text-embedding-3-small`.

See [Caching](/indexing/caching/) and [Storage](/indexing/storage/) for the node cache and the storage.

Chunking copies the metadata of a file to every chunk, so each chunk starts with the full outline and the questions of its file. The default embed mode embeds metadata together with the chunk. Compressing the outline keeps that embedded context small and specific.

## Cost per chunk

`MetadataQACode` runs before chunking here, so it sends one prompt per file and every chunk shares the answers. Move it after `then_chunk` to get questions per chunk, at one prompt per chunk. `CompressCodeOutline` sends one prompt per chunk of a file larger than the outline threshold. Chunks of smaller files have no outline and cost nothing. On a large codebase that is still many requests. Lower the concurrency or wrap the client in `LanguageModelWithBackOff` to stay within rate limits.

## Question prompt

`MetadataQACode` writes its reply to `Questions and Answers (code)`. The default prompt asks generic questions. The [ask-your-codebase](/showcases/ask-your-codebase/) showcase replaces it with a prompt about behaviour, which matches the questions developers ask:

::snippet{src="@swiftide/examples/ask-your-codebase/src/index.rs" anchor="questions_prompt" title="examples/ask-your-codebase/src/index.rs"}

The prompt receives `questions` and the node. The default prompt also renders `{{ outline }}`, which `MetadataQACode` sets when the node has an `Outline` field. The showcase prompt leaves it out, because the showcase builds no outline. Add `{{ outline }}` to your own prompt when you use it in the pipeline above, or the outline does not reach the LLM. [Tree-sitter](/integrations/tree-sitter/#questions-per-chunk) shows how to pass a prompt template and the number of questions.

## References and definitions

`MetadataRefsDefsCode` adds the symbols each chunk defines and uses, without an LLM:

::snippet{src="@snippets/src/integrations/tree_sitter.rs" anchor="refs_defs"}

It stores the symbols the chunk defines in `Definitions (code)` and the symbols it uses but does not define in `References (code)`. Both lists are sorted, without duplicates, and joined with commas. A field is left out when nothing is found. The constants `NAME_DEFINITIONS` and `NAME_REFERENCES` in `swiftide::indexing::transformers::metadata_refs_defs_code` hold the field names. Run it after chunking, so each chunk lists its own symbols.

## Languages

`ChunkCode`, `OutlineCodeTreeSitter` and `MetadataRefsDefsCode` take a language. `MetadataQACode` and `CompressCodeOutline` take only an LLM client. The constructor for references and definitions is `try_from_language`; the others use `try_for_language`. [Tree-sitter](/integrations/tree-sitter/#supported-languages) lists which languages each transformer supports. Perl needs the `tree-sitter-perl` feature flag. :unreleased

## Code splitter outside a pipeline

`CodeSplitter` is the splitter inside `ChunkCode`. Use it directly when you need chunks outside an indexing pipeline, for example in a tool:

::snippet{src="@snippets/src/indexing/code.rs" anchor="code_splitter"}

`SupportedLanguages` parses names without regard to case, and alternative names such as `"c#"`, `"c-sharp"`, `"cpp"` and `"c++"` work. Parsing an unknown name with `parse()` returns a strum parse error that says `Matching variant not found`. The constructors and `try_language` return the error `Treesitter language not supported` instead.

The chunk size is a `ChunkSize`. A number is a maximum in bytes; `ChunkSize::default()` is 1500 bytes. A range sets a minimum and a maximum, and a chunk is kept only when it is longer than the minimum. `CodeSplitter::new(language)` uses the default size.

## Gotchas

- The splitter drops chunks at or below the minimum and syntax leaves larger than the maximum, and returns a file with a syntax error as one chunk. [Chunking](/indexing/chunking/) covers these cases.
- `OutlineCodeTreeSitter` returns an error for a file with a syntax error. The first error aborts `run`, so add `log_errors()` and `filter_errors()` when your codebase contains files that do not parse. See [Streams and concurrency](/indexing/streams-and-concurrency/).
- `OutlineCodeTreeSitter` and `MetadataRefsDefsCode` panic on code in a language they do not handle. In a pipeline the panic happens inside a spawned task, so `run` fails with a join error.
- Chunk sizes for code are bytes. `ChunkMarkdown` and `ChunkText` count characters, see [Markdown and text](/indexing/chunking/#markdown-and-text).

## Reference

- [`ChunkCode`](docsrs:swiftide/indexing/transformers/struct.ChunkCode.html)
- [`OutlineCodeTreeSitter`](docsrs:swiftide/indexing/transformers/struct.OutlineCodeTreeSitter.html)
- [`CompressCodeOutline`](docsrs:swiftide/indexing/transformers/struct.CompressCodeOutline.html)
- [`MetadataQACode`](docsrs:swiftide/indexing/transformers/struct.MetadataQACode.html)
- [`MetadataRefsDefsCode`](docsrs:swiftide/indexing/transformers/struct.MetadataRefsDefsCode.html) and the [`metadata_refs_defs_code`](docsrs:swiftide/indexing/transformers/metadata_refs_defs_code/index.html) module with `NAME_DEFINITIONS` and `NAME_REFERENCES`
- [`CodeSplitter`](docsrs:swiftide/integrations/treesitter/struct.CodeSplitter.html)
- [`ChunkSize`](docsrs:swiftide/integrations/treesitter/enum.ChunkSize.html)
- [`SupportedLanguages`](docsrs:swiftide/integrations/treesitter/enum.SupportedLanguages.html)
- [`FileLoader`](docsrs:swiftide/indexing/loaders/struct.FileLoader.html)
- [`Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html)
- [`TextNode`](docsrs:swiftide/indexing/type.TextNode.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [Tree-sitter integration](/integrations/tree-sitter/) and [feature flags](/reference/feature-flags/)

## Next

[How querying works](/querying/overview/)
