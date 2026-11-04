---
title: "Glossary"
description: "One-line definitions of the core terms used across the Swiftide docs, each linked to the page that explains it."
sidebar:
  order: 6
---

You get a short definition of each core term in the Swiftide docs, grouped by section and linked to the page that explains it.

## Agents

- **[Agent](/agents/overview/)**: a loop around a large language model (LLM). Each turn, it sends the message history to the model and runs the tool calls it returns. Build one with `Agent::builder()`.
- **[Tool](/agents/tools/)**: a function the model can call by name with JSON arguments. It implements the `Tool` trait. The `#[tool]` macro and the `Tool` derive write it for you.
- **[Hook](/agents/hooks/)**: a closure the agent runs at a fixed point in its loop, such as `before_all`, `before_tool`, `after_each` or `on_stop`. Most hooks receive `&mut Agent`.
- **[Tool executor](/agents/executors/)**: where tools run commands and read or write files. It implements `ToolExecutor`. The default, `LocalExecutor`, runs commands on your machine.
- **[Context](/agents/context-and-history/)**: the state an agent runs in. It implements `AgentContext` and holds the message history and the tool executor. The default is `DefaultContext`.
- **[Message history](/agents/context-and-history/)**: the storage that keeps a context's chat messages. It implements `MessageHistory`. The default keeps messages in memory. `Redis` from the `redis` integration keeps them between runs.
- **[Stop reason](/agents/stopping-and-structured-results/)**: why an agent stopped, for example because a tool asked it to or it failed. Read it with `stop_reason()`. Its type is `StopReason`.

## Tasks

- **[Task](/tasks/overview/)**: a graph of typed steps with one input and one output type. Running a `Task` returns a `TaskRunOutcome`: the output, or paused.
- **[Task node](/tasks/nodes-and-transitions/)**: one typed step in a task, with an input, an output and an error type. It implements `TaskNode`. `Task::register_node_fn` turns a closure into a node.
- **[Transition](/tasks/nodes-and-transitions/)**: the value that says where a task goes after a node. Register it on that node with `Task::register_transition` and a function from the node's output to a `Transition`. In that function, `next.transitions_with(value)` moves to node `next`. Pass `task.transitions_to_finish()` as the function to end the task.
- **[Fan-out](/tasks/fan-out-and-join/)**: a transition that starts several branches at once. `Transition::fan_out` starts it, `and` adds a branch, and `join_with(join.join())` names the join that waits for every branch. Until then it is a `FanOutTransition`, which cannot run.
- **[Join](/tasks/fan-out-and-join/)**: a node that receives the outputs of every fan-out branch together as a `JoinInput`. `NodeId::join` gives the join target. Each branch registers its own transition to that target.

## Indexing

- **[Indexing pipeline](/indexing/overview/)**: a stream that loads data as nodes, transforms them and stores them. Start one with `Pipeline::from_loader` from `swiftide::indexing` and run it with `run`.
- **[Node](/indexing/overview/)**: one unit of data in an indexing pipeline: a path, a chunk, metadata and embeddings. `Node` is generic over its chunk type. `TextNode` is a node whose chunk is a `String`.
- **[Chunk](/indexing/chunking/)**: the content a node carries, in its `chunk` field. Any type that is `Clone`, `Send`, `Sync`, `Debug`, `'static` and readable as bytes implements the `Chunk` trait automatically.
- **[Loader](/indexing/loading/)**: the source at the start of an indexing pipeline. It implements `Loader` and turns files, pages or messages into nodes. `FileLoader` reads files from disk.
- **[Transformer](/indexing/transformers-and-metadata/)**: a step that maps one node to one node, for example to add metadata. It implements `Transformer`. Add it with `then`.
- **[Batch transformer](/indexing/transformers-and-metadata/)**: a step that takes many nodes in one call. It implements `BatchableTransformer`. Add it with `then_in_batch`.
- **[Chunker](/indexing/chunking/)**: a step that splits one node into many. It implements `ChunkerTransformer`, and `ChunkMarkdown` is one. Add it with `then_chunk`.
- **[Embedder](/indexing/embedding/)**: a model that turns text into vectors. Dense models implement `EmbeddingModel` and `Embed` calls them. Sparse models implement `SparseEmbeddingModel` and `SparseEmbed` calls them.
- **[Persist](/indexing/storage/)**: the trait a pipeline calls to store nodes. Add a storage with `then_store_with`.
- **[Storage](/indexing/storage/)**: the backend that implements `Persist`, such as Qdrant, PgVector, Redis or LanceDB. Their flags are `qdrant`, `pgvector`, `redis` and `lancedb`.
- **[Node cache](/indexing/caching/)**: a record of the nodes a pipeline already processed. It implements `NodeCache`. `filter_cached` drops nodes found in the cache. A node enters it only after passing the whole pipeline.

## Querying

- **[Query pipeline](/querying/overview/)**: a pipeline that turns a question into an answer in four stages: query transformers (`then_transform_query`), a retriever (`then_retrieve`), response transformers (`then_transform_response`) and an answer generator (`then_answer`). Start one with `Pipeline::from_search_strategy` from `swiftide::query` and ask with `query`.
- **[Retriever](/querying/retrieval-and-search-strategies/)**: a store that fetches documents with a search strategy. It implements `Retrieve` for that strategy. Qdrant, PgVector, LanceDB (`lancedb` flag) and DuckDB (`duckdb` flag) are retrievers.
- **[Search strategy](/querying/retrieval-and-search-strategies/)**: settings for how a retriever searches, such as result count and filter. It implements `SearchStrategy`. Built-in ones are `SimilaritySingleEmbedding`, `HybridSearch` and `CustomStrategy`.
- **[Response transformer](/querying/reranking-and-response-transformers/)**: a step between retrieval and the answer. It implements `TransformResponse`. `Summary` replaces the response with a summary of the documents. The FastEmbed `Rerank` (`fastembed` flag) reorders the documents.
- **[Answer generator](/querying/answering/)**: the last step, which writes the answer from the documents. It implements `Answer`. `Simple` from `swiftide::query::answers` asks an LLM.
- **[Evaluator](/querying/evaluation/)**: a component that sees the retrieved documents and the final answer of each query. It implements `EvaluateQuery`. Add it with `evaluate_with`. `Ragas` exports a dataset for the Ragas framework.

## Setup

- **[Feature flag](/reference/feature-flags/)**: a Cargo feature of `swiftide` that compiles in an optional part. Only `macros` is on by default. Agents need `swiftide-agents`, tasks need `swiftide-tasks`, and each integration needs its own flag.
- **[Integration](/integrations/overview/)**: a module under `swiftide::integrations` that wraps an outside library or service. Examples are providers (`openai`), stores (`qdrant`), loaders (`scraping`) and parsers (`treesitter`). Each has its own feature flag.

## Reference

- [`swiftide`](docsrs:swiftide) (crate root, with the list of Cargo features)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html), [`Hook`](docsrs:swiftide/agents/hooks/enum.Hook.html), [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html), [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html), [`LocalExecutor`](docsrs:swiftide/agents/tools/local_executor/struct.LocalExecutor.html)
- [`Tool`](docsrs:swiftide/chat_completion/trait.Tool.html), [`ToolExecutor`](docsrs:swiftide/traits/trait.ToolExecutor.html), [`AgentContext`](docsrs:swiftide/traits/trait.AgentContext.html), [`MessageHistory`](docsrs:swiftide/traits/trait.MessageHistory.html)
- [`Task`](docsrs:swiftide/tasks/struct.Task.html), [`TaskRunOutcome`](docsrs:swiftide/tasks/enum.TaskRunOutcome.html), [`TaskNode`](docsrs:swiftide/tasks/trait.TaskNode.html), [`Transition`](docsrs:swiftide/tasks/struct.Transition.html), [`FanOutTransition`](docsrs:swiftide/tasks/struct.FanOutTransition.html), [`NodeId`](docsrs:swiftide/tasks/struct.NodeId.html), [`JoinInput`](docsrs:swiftide/tasks/struct.JoinInput.html)
- [`indexing::Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html), [`Node`](docsrs:swiftide/indexing/struct.Node.html), [`TextNode`](docsrs:swiftide/indexing/type.TextNode.html), [`Chunk`](docsrs:swiftide/indexing/trait.Chunk.html), [`Loader`](docsrs:swiftide/traits/trait.Loader.html), [`Transformer`](docsrs:swiftide/traits/trait.Transformer.html), [`BatchableTransformer`](docsrs:swiftide/traits/trait.BatchableTransformer.html), [`ChunkerTransformer`](docsrs:swiftide/traits/trait.ChunkerTransformer.html), [`ChunkMarkdown`](docsrs:swiftide/indexing/transformers/struct.ChunkMarkdown.html), [`EmbeddingModel`](docsrs:swiftide/traits/trait.EmbeddingModel.html), [`SparseEmbeddingModel`](docsrs:swiftide/traits/trait.SparseEmbeddingModel.html), [`Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html), [`SparseEmbed`](docsrs:swiftide/indexing/transformers/struct.SparseEmbed.html), [`Persist`](docsrs:swiftide/traits/trait.Persist.html), [`NodeCache`](docsrs:swiftide/traits/trait.NodeCache.html)
- [`query::Pipeline`](docsrs:swiftide/query/struct.Pipeline.html), [`Retrieve`](docsrs:swiftide/traits/trait.Retrieve.html), [`SearchStrategy`](docsrs:swiftide/traits/trait.SearchStrategy.html), [`SimilaritySingleEmbedding`](docsrs:swiftide/query/search_strategies/struct.SimilaritySingleEmbedding.html), [`HybridSearch`](docsrs:swiftide/query/search_strategies/struct.HybridSearch.html), [`CustomStrategy`](docsrs:swiftide/query/search_strategies/struct.CustomStrategy.html), [`TransformResponse`](docsrs:swiftide/traits/trait.TransformResponse.html), [`Summary`](docsrs:swiftide/query/response_transformers/struct.Summary.html), [`Answer`](docsrs:swiftide/traits/trait.Answer.html), [`Simple`](docsrs:swiftide/query/answers/struct.Simple.html), [`EvaluateQuery`](docsrs:swiftide/traits/trait.EvaluateQuery.html), [`Ragas`](docsrs:swiftide/query/evaluators/ragas/struct.Ragas.html)
- [`swiftide::integrations`](docsrs:swiftide/integrations/index.html)

Next: [Contributing and community](/community/contributing/)
