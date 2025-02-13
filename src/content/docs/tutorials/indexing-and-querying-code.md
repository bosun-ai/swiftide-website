---
title: Indexing and querying code
description: Learn how to build an application that can answer questions about your code
---

In this tutorial, we will cover the basics of indexing and querying code with Swiftide. Step by step we will set up the required dependencies and build up a pipeline. We will then manually build a query function to answer a user's question.

The full code is available on the [github tutorial repository](https://github.com/bosun-ai/swiftide-tutorial/tree/master/indexing-and-querying-code).

## Setting up the project

Before we can get started we need to set up a new Rust project and install Redis and Qdrant.

Make sure you have the Rust toolchain installed, [rustup](https://rustup.rs/) is recommended. For the example we will use OpenAI to enrich, embed and query our data.

Then install both Redis and Qdrant, either via your favourite package manager or run them as one-off via Docker.
On [github](https://github.com/bosun-ai/swiftide-tutorial/tree/master/indexing-and-querying-code) you can also find a docker compose file.

Then make sure the following environment variables are configured:

```shell
export OPENAI_API_KEY=<openai api key>
export REDIS_URL=<redis url, i.e. redis://localhost:6379>
export QDRANT_URL=<grpc qdrant url, i.e. http://localhost:6334>
```

### Creating the Rust project

First, let's set up the Rust project:

```shell
$ cargo new swiftide-example
$ cd swiftide-example
```

Next we'll add some useful dependencies to make our life easier, and Tokio:

```shell
$ cargo add anyhow clap tokio --features tokio/full,clap/derive
```

And add Swiftide itself:

```shell
$ cargo add swiftide --features qdrant,redis,openai,tree-sitter
```

### Setting up our main function

Then set up a basic `main` for the project:

```rust
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
  println!("Hello Swiftide");

  Ok(())
}
```

Give it a test:

```shell
$ cargo run

Hello Swiftide
```

And we're ready to go!

## Building the indexing pipeline

Any RAG pipeline is only as good as the data it ingests. For brevity, let's assume the project is written in a single language and has some markdown like a readme. In bigger projects we might want to index documentation, websites and other code as well. That's all possible with Swiftide, but let's keep that out of scope for now.

### Getting user input

First, let's get some user input when we run the program. We want to know the language, the directory it should index and query, and later the query itself. Let's use `clap` for this, which we added earlier.

And add the argument parsing:

```rust
use clap::Parser;
use anyhow::Result;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    language: String,

    #[arg(short, long, default_value_t = ".")]
    path: PathBuf,

    query: String
}

#[tokio::main]
async fn main() -> Result<()> {
  let args = Args::parse();

  println!("{}", args.language);
  println!("{}", args.path.to_string_lossy());
  println!("{}", args.query);
}
```

Let's run it to make sure it works:

```shell
$ cargo run -- --help
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
     Running `target/debug/swiftide-example --help`
Usage: swiftide-example [OPTIONS] --language <LANGUAGE> <QUERY>

Arguments:
  <QUERY>

Options:
  -l, --language <LANGUAGE>
  -p, --path <PATH>          [default: ./]
  -h, --help                 Print help
  -V, --version              Print version

$ cargo run -- --language rust "How do I index code with Swiftide?"
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
     Running `target/debug/swiftide-example --language rust 'How do I index code with Swiftide?'`
rust
./
How do I index code with Swiftide?
```

### Enabling logging

Before we continue, let's enable logging so we can easily see if something goes wrong.

```shell
$ cargo add tracing-subscriber
```

And initialize the stdout subscriber right below main:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    ...
```

By setting `RUST_LOG` to debug or info it should now be a lot easier to identify problems.

### Indexing markdown files

First we will index any markdown files, like a README. There might be valuable data about what the project does and we want to be sure to have that included.

```rust
async fn index_markdown(path: &PathBuf) -> Result<()> {
    tracing::info!(path=?path, "Indexing markdown");

    // Loads all markdown files into the pipeline
    Pipeline::from_loader(FileLoader::new(path).with_extensions(&["md"]))
        .run()
        .await
}
```

And call it from `main`:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    index_markdown(&args.path).await?;

    Ok(())
}
```

This loads all markdown files into the pipeline and runs the pipeline, but it does not do anything yet.

### Chunking the data

In RAG, we generally store smaller chunks than the full file. This gives more granular and detailed results and gives the opportunity to very specifically enrich. Chunking is an exercise by itself. The goal is to get small, meaningful blocks while still allowing for performance at scale.

For this we will use the markdown chunker, which under the hood uses the excellent Rust `text-splitter` crate. It tries to get meaningful blocks of text by using the markdown formatting as a guide.

```rust
async fn index_markdown(path: &PathBuf) -> Result<()> {
    tracing::info!(path=?path, "Indexing markdown");

    Pipeline::from_loader(FileLoader::new(path).with_extensions(&["md"]))
        // The range ensures that chunks smaller than 50 characters are dropped, with a maximum size up to 1024 characters
        .then_chunk(ChunkMarkdown::from_chunk_range(50..1024))
        .run()
        .await
}
```

### Enriching the data with questions and answers

A common technique for RAG is to generate questions per chunk. When searching the index, this gives more overlap with queries.

Let's use a transformer with defaults to add it. This also sets up `openai` as the llm. OpenAI is cheap to clone, it wraps the client with an internal `Arc`.

```rust
#[tokio::main]
async fn main() -> Result<()> {
    ...

    let openai = OpenAI::builder()
        .default_embed_model("text-embedding-3-small")
        .default_prompt_model("gpt-3.5-turbo")
        .build()?;

    index_markdown(&args.path, &openai).await?;

    Ok(())
}

async fn index_markdown(path: &PathBuf, openai: &OpenAI) -> Result<()> {
    tracing::info!(path=?path, "Indexing markdown");

    // Loads all markdown files into the pipeline
    Pipeline::from_loader(FileLoader::new(path).with_extensions(&["md"]))
        // The range ensures that chunks smaller than 50 characters are dropped, with a maximum size up to 1024 characters
        .then_chunk(ChunkMarkdown::from_chunk_range(50..1024))
        // Generate questions and answers and them to the metadata of the node
        .then(MetadataQAText::new(openai.clone()))
        .run()
        .await
}
```

When you try to run the pipeline, it will fail with an error that no storage is set. A pipeline must always end in at least one storage.

### Embedding and storing the data

Next we will embed the chunks and store them into Qdrant. The `Embed` transformer only supports batch mode. Depending on the LLM, chunk sizes and resources available; this is a potential parameter to tune.

```rust

#[tokio::main]
async fn main() -> Result<()> {
    ...

    let qdrant = Qdrant::builder()
        .vector_size(1536)
        .collection_name("swiftide-tutorial")
        .build()?;

    index_markdown(&args.path, &openai, &qdrant).await?;

    Ok(())
}

async fn index_markdown(path: &PathBuf, openai: &OpenAI, qdrant: &Qdrant) -> Result<()> {
    tracing::info!(path=?path, "Indexing markdown");

    // Loads all markdown files into the pipeline
    Pipeline::from_loader(FileLoader::new(path).with_extensions(&["md"]))
        // The range ensures that chunks smaller than 50 characters are dropped, with a maximum size up to 1024 characters
        .then_chunk(ChunkMarkdown::from_chunk_range(50..1024))
        // Generate questions and answers and them to the metadata of the node
        .then(MetadataQAText::new(openai.clone()))
        // Embed chunks in batches of 100. By default the metadata in the node is included in the
        // embedding
        .then_in_batch(Embed::new(openai.clone()))
        // Finally store the embeddings into Qdrant
        .then_store_with(qdrant.clone())
        .run()
        .await
}
```

Great! Let's run this on a repository and check if it works:

### Indexing code

For indexing code we will do the exact same process in a new pipeline.

```rust

#[tokio::main]
async fn main() -> Result<()> {
    ...
    index_markdown(&args.path, &openai, &qdrant).await?;
    index_code(&args.language, &args.path, &openai, &qdrant).await?;

    Ok(())
}

async fn index_code(
    language: &str,
    path: &PathBuf,
    openai: &OpenAI,
    qdrant: &Qdrant,
) -> Result<()> {
    tracing::info!(path=?path, language, "Indexing code");

    // Parse the language to a supported one by the tree-sitter integration. Clap could also do
    // this. It's also possible to directly pass the language as string to the transformer, but we
    // save some time by getting the extensions.
    let language = SupportedLanguages::from_str(language)?;

    Pipeline::from_loader(FileLoader::new(path).with_extensions(language.file_extensions()))
        // Uses tree-sitter to extract best effort blocks of code. We still keep the minimum
        // fairly high and double the chunk size
        .then_chunk(ChunkCode::try_for_language_and_chunk_size(
            language,
            50..2048,
        )?)
        .then(MetadataQACode::new(openai.clone()))
        .then_in_batch(Embed::new(openai.clone()))
        .then_store_with(qdrant.clone())
        .run()
        .await
}
```

Let's give it a whirl:

```shell
$ time cargo run -- --language rust --path '../swiftide' some-query
2024-07-14T16:31:43.900154Z  INFO swiftide_example: Indexing markdown path="../swiftide"
2024-07-14T16:31:43.903751Z  INFO indexing_pipeline.run: swiftide::indexing::pipeline: Starting indexing pipeline with 12 concurrency
2024-07-14T16:31:43.903810Z  INFO indexing_pipeline.run:setup: swiftide::integrations::qdrant: Checking if collection swiftide-tutorial exists
2024-07-14T16:31:43.907118Z  WARN indexing_pipeline.run:setup: swiftide::integrations::qdrant: Collection swiftide-tutorial exists
2024-07-14T16:32:10.209819Z  WARN indexing_pipeline.run: swiftide::indexing::pipeline: Processed 207 nodes
2024-07-14T16:32:10.209979Z  INFO swiftide_example: Indexing code path="../swiftide" language="rust"
2024-07-14T16:32:10.211053Z  INFO indexing_pipeline.run: swiftide::indexing::pipeline: Starting indexing pipeline with 12 concurrency
2024-07-14T16:32:10.211084Z  INFO indexing_pipeline.run:setup: swiftide::integrations::qdrant: Checking if collection swiftide-tutorial exists
2024-07-14T16:32:10.211896Z  WARN indexing_pipeline.run:setup: swiftide::integrations::qdrant: Collection swiftide-tutorial exists
2024-07-14T16:33:06.964750Z  WARN indexing_pipeline.run: swiftide::indexing::pipeline: Processed 271 nodes
cargo run -- --language rust --path '../swiftide' test  1.94s user 0.20s system 2% cpu 1:23.28 total
```

1 minute and 23 seconds. Not bad!

### Getting swifty

Let's see if we can speed it up. Since requests to openai take time, we can increase the concurrency (default is number of cpus) up to whatever the rate limit allows.

There are also approximately 500 nodes to process while we batch in 100, which means we can experiment with smaller batches to get higher parallelism.

Finally, we could also split a single stream into markdown and code. Let's do all at once and see where we're at:

```rust

#[tokio::main]
async fn main() -> Result<()> {
    ...
    index_all(&args.language, &args.path, &openai, &qdrant).await?;

    Ok(())
}

async fn index_all(language: &str, path: &PathBuf, openai: &OpenAI, qdrant: &Qdrant) -> Result<()> {
    tracing::info!(path=?path, language, "Indexing code");

    let language = SupportedLanguages::from_str(language)?;
    let mut extensions = language.file_extensions().to_owned();
    extensions.push("md");

    let (mut markdown, mut code) =
        Pipeline::from_loader(FileLoader::new(path).with_extensions(&extensions))
            // At most the pipelines will do 50 concurrent requests
            .with_concurrency(50)
            // Split the pipeline based on the extension in the path
            .split_by(|node| {
                // Any errors at this point we just pass to 'markdown'
                let Ok(node) = node else { return true };

                // On true we go 'markdown', on false we go 'code'.
                node.path.extension().map_or(true, |ext| ext == "md")
            });

    code = code
        // Uses tree-sitter to extract best effort blocks of code. We still keep the minimum
        // fairly high and double the chunk size
        .then_chunk(ChunkCode::try_for_language_and_chunk_size(
            language,
            50..1024,
        )?)
        .then(MetadataQACode::new(openai.clone()));

    markdown = markdown
        .then_chunk(ChunkMarkdown::from_chunk_range(50..1024))
        // Generate questions and answers and them to the metadata of the node
        .then(MetadataQAText::new(openai.clone()));

    code.merge(markdown)
        .then_in_batch(Embed::new(openai.clone()))
        .then_store_with(qdrant.clone())
        .run()
        .await
}
```

And let's give it another whirl:

```shell
time cargo run -- --language rust --path '../swiftide' test
2024-07-14T16:56:56.100102Z  INFO swiftide_example: Indexing code path="../swiftide" language="rust"
2024-07-14T16:56:56.103657Z  INFO indexing_pipeline.run: swiftide::indexing::pipeline: Starting indexing pipeline with 50 concurrency
2024-07-14T16:56:56.103761Z  INFO indexing_pipeline.run:setup: swiftide::integrations::qdrant: Checking if collection swiftide-tutorial exists
2024-07-14T16:56:56.108963Z  WARN indexing_pipeline.run:setup: swiftide::integrations::qdrant: Collection swiftide-tutorial exists
2024-07-14T16:57:11.984648Z  WARN indexing_pipeline.run: swiftide::indexing::pipeline: Processed 478 nodes
cargo run -- --language rust --path '../swiftide' test
4.53s user 0.74s system 30% cpu 17.143 total
```

Aww yeah, _478 nodes processed in 17 seconds_! For the record, splitting gets it down to ~50 seconds, and bumping the currency gets it down close to 20. The smaller embedding batches shave off the rest.

With that sorted, let's quickly add a node cache right before splitting, so that subsequent queries will only re-index changed nodes:

```rust
.with_concurrency(50)
.filter_cached(Redis::try_from_url(
    "redis://localhost:6379",
    "swiftide-tutorial",
)?)
.split_by(...)
```

## Querying our data

```rust
#[tokio::main]
async fn main() -> Result<()> {
    ...

    let answer = query(&args.query, &openai, &qdrant).await?.answer();
    println!("{answer}");

    Ok(())
}


fn query(query: &str, openai_client: &OpenAI, qdrant: &Qdrant) -> Result<Query<states::Answered>> {
    // Create a query pipeline that uses the default similarity search
    let pipeline = query::Pipeline::default()
        // Generate subquestions to improve the semantic coverage
        .then_transform_query(query_transformers::GenerateSubquestions::from_client(
            openai_client.clone(),
        ))
        // Then embed the transformed query
        .then_transform_query(query_transformers::Embed::from_client(
            openai_client.clone(),
        ))
        // Retrieve documents from qdrant
        .then_retrieve(qdrant.clone())
        // Summarize all the retrieved documents
        .then_transform_response(response_transformers::Summary::from_client(
            openai_client.clone(),
        ))
        // Use the summary as context for answering our question
        .then_answer(answers::Simple::from_client(openai_client.clone()));

    pipeline
        .query("What is swiftide? Please provide an elaborate explanation")
        .await
}
```

And let's give it a final go:

```shell
$ cargo run -- --language rust --path '../swiftide' "What is swiftide? Please provide an elaborate explanation"
```

Which gives us the following answer:

```markdown
Based on the provided context, Swiftide can be described as follows:

# Swiftide Overview

Swiftide is a Rust-native library designed for building Large Language Model (LLM) applications. It facilitates the ingestion, transformation, and indexing of large volumes of data quickly, making it possible to query this data and use it in prompts. This process is known as Retrieval Augmented Generation.

## Features and Goals

Swiftide aims to provide a fast, easy-to-use, reliable, and extendable library. It was developed to overcome performance and stability issues encountered with Python-based tools. The library is part of the upcoming [bosun.ai](https://bosun.ai) platform for autonomous code improvement.

**Key Features:**

- Fast and efficient data processing
- Built with Rust for performance and reliability
- Easy-to-integrate with other AI tools and libraries
- Open for community contributions and feedback

### Installation

To install Swiftide, you need to set up a new Rust project and add Swiftide to your project using the following command: `cargo add swiftide`

For more detailed information about getting started, indexing, querying, and contributing to the project, you can refer to the official documentation and resources provided.

Swiftide is distributed under the MIT License, ensuring it is freely available for use and modification."
```

Swiftide offers a lot more features for customization and tuning. Every RAG application is different, with different data, and different requirements.

For more in depth documentation, check out [our api documentation](https://docs.rs/swiftide) and the rest of the documentation. Questions? Join us on Discord!

[1]: https://swiftide.rs
