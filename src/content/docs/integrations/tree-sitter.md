---
title: "Tree-sitter"
description: "Split source code along syntax boundaries with ChunkCode and add outlines, questions, references and definitions as metadata, behind the tree-sitter feature flag."
sidebar:
  order: 10
---

With this integration your indexing pipeline splits source files along functions, classes and other syntax boundaries instead of at a fixed length, and adds code-aware metadata to every chunk. Tree-sitter is a parser library with a grammar per language. Swiftide ships the grammars and a set of transformers that use them.

## Feature flag

```sh
cargo add swiftide@{{swiftide_version}} --features tree-sitter
```

The `tree-sitter` feature flag adds the grammars and the transformers `ChunkCode`, `MetadataQACode`, `MetadataRefsDefsCode`, `OutlineCodeTreeSitter` and `CompressCodeOutline`. They are re-exported at `swiftide::indexing::transformers`, next to the other transformers. `SupportedLanguages` sits at `swiftide::integrations::treesitter`.

The `tree-sitter-perl` feature flag adds Perl and turns on `tree-sitter` too. :unreleased

## Supported languages

::snippet{src="@snippets/src/integrations/tree_sitter.rs" anchor="languages"}

`SupportedLanguages` implements `FromStr` and ignores case. `ChunkCode`, `OutlineCodeTreeSitter` and `MetadataRefsDefsCode` take a `SupportedLanguages` value or a string such as `"rust"`, and return an error that says `Treesitter language not supported` for a name they do not know. `MetadataQACode` and `CompressCodeOutline` take no language, only an LLM client. `file_extensions()` gives the extensions to pass to `FileLoader`.

| Language | Names | Extensions | Outline | References and definitions |
| --- | --- | --- | --- | --- |
| Rust | `rust` | rs | yes | yes |
| TypeScript | `typescript` | ts, tsx, js, jsx | yes | yes |
| JavaScript | `javascript` | js, jsx | yes | yes |
| Python | `python` | py | yes | yes |
| Ruby | `ruby` | rb | yes | yes |
| Java | `java` | java | yes | yes |
| C# | `csharp`, `c-sharp`, `c#` | cs, csx | yes | yes |
| Go | `go` | go | no | yes |
| Solidity | `solidity` | sol | no | yes |
| C | `c` | c, h, o | no | no |
| C++ | `cpp`, `c++` | c, h, o, cc, cpp | no | no |
| Elixir | `elixir` | ex, exs | no | no |
| HTML | `html` | html, htm, xhtml | no | no |
| PHP | `php` | php | no | no |
| Perl :unreleased | `perl` | pm, pl, t | no | no |

Every language works with `ChunkCode`. The last two columns say which languages `OutlineCodeTreeSitter` and `MetadataRefsDefsCode` handle.

## Chunk size and splitting

::snippet{src="@swiftide/examples/index_codebase.rs" anchor="pipeline"}

The example also uses the `redis`, `qdrant` and `openai` feature flags, and needs a running Redis server for the node cache. See [feature flags](/reference/feature-flags/).

`ChunkCode` is a chunker. It parses each file and walks the syntax tree, adding top-level items to a chunk. It closes the chunk when the next item would bring it to the maximum size or past it. An item larger than the maximum is split along its own children, one level down at a time.

The chunk size is a `ChunkSize`. A number, such as `1000`, is a maximum in bytes. A range, such as `10..2048`, sets a minimum and a maximum: a chunk no larger than the minimum is dropped. `ChunkCode::try_for_language` uses a maximum of 1500 bytes. `with_concurrency` sets how many files are chunked at once.

## Several languages in one pipeline

`ChunkCode` handles one language. To index several, pick a `ChunkCode` per file extension, as the ask-your-codebase showcase does with its `ChunkByLanguage` chunker (see [Chunking](/indexing/chunking/)). The first language that lists an extension wins, so the order of the list matters where extensions overlap.

## Questions per chunk

::snippet{src="@swiftide/examples/ask-your-codebase/src/index.rs" anchor="pipeline" title="examples/ask-your-codebase/src/index.rs" mark="16-22"}

The showcase builds swiftide with the `tree-sitter`, `fastembed`, `lancedb`, `redb` and `openai` feature flags.

`MetadataQACode` asks a large language model (LLM) which questions the code answers and stores the reply under the metadata key `Questions and Answers (code)`. A question at query time then finds the chunk more easily, because the questions are embedded with the code. Run it before chunking to send one prompt per file, which every chunk of that file then inherits. Run it after chunking to get questions per chunk, at one prompt per chunk. It writes 5 questions by default. `MetadataQACode::new(client)` builds it with the defaults, and `from_client(client)` returns a builder for `prompt_template`, `num_questions` and `concurrency`.

The showcase runs it after chunking, so each chunk gets its own questions. It also replaces the default prompt with one that asks about behaviour, defined at the top of the same file. When a node has `Outline` metadata, the default prompt includes the outline.

The showcase's run, with `claude-haiku-5-5` writing the questions and FastEmbed embedding on the CPU, indexed the `swiftide-agents` crate like this:

```text
Indexed 34 chunks from 14 new or changed files (14 files found, 0 unchanged) in 91.9s
```

## Outlines, references and definitions

`OutlineCodeTreeSitter`, `CompressCodeOutline` and `MetadataRefsDefsCode` give each chunk an outline of its file and the symbols it defines and uses. For pipelines that add them to every chunk, see [Code-aware indexing](/indexing/code-aware-indexing/).

## Gotchas

- `OutlineCodeTreeSitter` and `MetadataRefsDefsCode` build for every language, but panic on a node in a language they do not handle. `OutlineCodeTreeSitter` skips nodes below its minimum file size first. The panic happens inside a spawned task, so `run()` fails with a join error instead of crashing the process. Check the table before you use them.
- When a file has a syntax error, `ChunkCode` logs a warning and returns the whole file as one chunk, whatever its size. `OutlineCodeTreeSitter` returns an error for that node, and the first error stops the whole pipeline: `run()` fails. Add `filter_errors()` after it when your codebase may contain files that do not parse. See [Streams and concurrency](/indexing/streams-and-concurrency/).
- `SupportedLanguages::Typescript` lists `tsx`, `js` and `jsx` in `file_extensions()`, but uses the plain TypeScript grammar, which cannot parse JSX. A `.tsx` or `.jsx` file that contains JSX has a syntax error, with the consequences above. Put JavaScript before TypeScript when you pick a language by extension, so `js` and `jsx` go to the JavaScript grammar.
- The C and C++ lists both include `c`, `h` and `o`. `o` is the extension of compiled object files, so a `FileLoader` built from them picks up build output.
- `MetadataQACode` and `CompressCodeOutline` send one prompt per node. On a large codebase that is many requests; lower the concurrency or wrap the client in `LanguageModelWithBackOff` to stay within rate limits.

## Reference

- [`SupportedLanguages`](docsrs:swiftide/integrations/treesitter/enum.SupportedLanguages.html)
- [`ChunkCode`](docsrs:swiftide/integrations/treesitter/chunk_code/struct.ChunkCode.html)
- [`ChunkSize`](docsrs:swiftide/integrations/treesitter/enum.ChunkSize.html)
- [`MetadataQACode`](docsrs:swiftide/integrations/treesitter/metadata_qa_code/struct.MetadataQACode.html)
- [`MetadataRefsDefsCode`](docsrs:swiftide/integrations/treesitter/metadata_refs_defs_code/struct.MetadataRefsDefsCode.html)
- [`OutlineCodeTreeSitter`](docsrs:swiftide/integrations/treesitter/outline_code_tree_sitter/struct.OutlineCodeTreeSitter.html)
- [`CompressCodeOutline`](docsrs:swiftide/integrations/treesitter/compress_code_outline/struct.CompressCodeOutline.html)
- [`FileLoader`](docsrs:swiftide/indexing/loaders/struct.FileLoader.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [Code-aware indexing](/indexing/code-aware-indexing/)

## Next

[Ask your codebase](/showcases/ask-your-codebase/)
