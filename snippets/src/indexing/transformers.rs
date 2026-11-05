//! Snippets for `indexing/transformers-and-metadata`.

use std::collections::HashSet;

use async_trait::async_trait;
use swiftide::{
    indexing::{
        IndexingStream, Pipeline, TextNode,
        loaders::FileLoader,
        persist::MemoryStorage,
        transformers::{ChunkMarkdown, MetadataQAText, MetadataSummary, MetadataTitle},
    },
    integrations::openai::OpenAI,
    traits::{BatchableTransformer, Transformer, WithBatchIndexingDefaults, WithIndexingDefaults},
};

/// One client for every LLM transformer, set once on the pipeline.
pub async fn default_llm_client(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: default_llm_client
    Pipeline::from_loader(FileLoader::new("docs").with_extensions(&["md"]))
        // Set the client before the steps that use it
        .with_default_llm_client(openai)
        .then_chunk(ChunkMarkdown::from_chunk_range(100..1500))
        .then(MetadataTitle::default())
        .then(MetadataSummary::default())
        .then(
            MetadataQAText::builder()
                .num_questions(3_usize)
                .concurrency(4_usize)
                .build()?,
        )
        .then_store_with(MemoryStorage::default())
        .run()
        .await
    // ANCHOR_END: default_llm_client
}

// ANCHOR: custom_transformer
/// Stores the number of words in a chunk as the `Words` metadata field.
#[derive(Clone)]
pub struct WordCount;

#[async_trait]
impl Transformer for WordCount {
    type Input = String;
    type Output = String;

    async fn transform_node(&self, mut node: TextNode) -> anyhow::Result<TextNode> {
        let words = node.chunk.split_whitespace().count();
        node.metadata.insert("Words", words);
        Ok(node)
    }
}

// `then` hands pipeline defaults to every transformer; this one has no use for them.
impl WithIndexingDefaults for WordCount {}
// ANCHOR_END: custom_transformer

// ANCHOR: batch_transformer
/// Drops chunks that repeat an earlier chunk in the same batch, such as copied licence headers.
#[derive(Clone)]
pub struct DropRepeatedChunks {
    batch_size: usize,
}

#[async_trait]
impl BatchableTransformer for DropRepeatedChunks {
    type Input = String;
    type Output = String;

    async fn batch_transform(&self, nodes: Vec<TextNode>) -> IndexingStream<String> {
        let mut seen = HashSet::new();
        let unique: Vec<TextNode> = nodes
            .into_iter()
            .filter(|node| seen.insert(node.chunk.trim().to_owned()))
            .collect();
        unique.into()
    }

    fn batch_size(&self) -> Option<usize> {
        Some(self.batch_size)
    }
}

impl WithBatchIndexingDefaults for DropRepeatedChunks {}
// ANCHOR_END: batch_transformer

/// Both custom transformers in a pipeline.
pub async fn custom_steps() -> anyhow::Result<()> {
    // ANCHOR: custom_steps
    Pipeline::from_loader(FileLoader::new("docs").with_extensions(&["md"]))
        .then_chunk(ChunkMarkdown::from_chunk_range(100..1500))
        .then_in_batch(DropRepeatedChunks { batch_size: 64 })
        .then(WordCount)
        .then_store_with(MemoryStorage::default())
        .run()
        .await
    // ANCHOR_END: custom_steps
}

// ANCHOR: macro_transformer
/// The macro adds items next to the struct, so each annotated struct gets its own module.
pub mod metadata_audience {
    use async_trait::async_trait;
    use swiftide::{indexing::TextNode, indexing_transformer, traits::Transformer};

    /// Asks the LLM who a chunk is written for and stores the reply as `Audience`.
    #[indexing_transformer(
        metadata_field_name = "Audience",
        default_prompt_file = "prompts/metadata_audience.prompt.md"
    )]
    pub struct MetadataAudience {
        /// Product name passed to the prompt as `{{ product }}`.
        #[builder(default = "\"swiftide\".to_string()")]
        product: String,
    }

    #[async_trait]
    impl Transformer for MetadataAudience {
        type Input = String;
        type Output = String;

        async fn transform_node(&self, mut node: TextNode) -> anyhow::Result<TextNode> {
            // The macro renders nothing for you: add the node and your own values here.
            let prompt = self
                .prompt_template
                .clone()
                .with_node(&node)
                .with_context_value("product", self.product.as_str());

            // `prompt` uses this transformer's client, or the pipeline default.
            let audience = self.prompt(prompt).await?;
            node.metadata.insert(NAME, audience);
            Ok(node)
        }

        // The macro stores `with_concurrency(n)` but does not apply it.
        fn concurrency(&self) -> Option<usize> {
            self.concurrency
        }
    }
}
// ANCHOR_END: macro_transformer
