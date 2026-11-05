---
title: "Streaming loaders"
description: "Index messages from a Kafka or Fluvio topic as they arrive, and store the embedded nodes in a second Kafka topic or in Qdrant."
sidebar:
  order: 10
identLint:
  ignore: [ClientConfig, ConsumerConfigExt, FluvioConfig, "Offset::from_end(1)", fluvio_key]
---

These two recipes give you an indexing pipeline that reads a message topic and keeps running as new messages arrive. The first reads a Kafka topic, embeds each message and writes the result to a second Kafka topic. The second reads a Fluvio topic and stores the embeddings in Qdrant. [Loading](/indexing/loading/#kafka-and-fluvio) covers the loaders themselves; this page runs them end to end.

## Kafka

Add Swiftide with the `kafka` and `fastembed` feature flags, plus Tokio for the async runtime:

```sh
cargo add swiftide@{{swiftide_version}} --features kafka,fastembed
cargo add tokio --features macros,rt-multi-thread
```

The `kafka` feature flag builds librdkafka from source through rdkafka's `cmake-build` feature, so the build machine needs CMake and a C compiler.

### Kafka loader and storage

::snippet{src="@swiftide/examples/kafka.rs" anchor="setup" title="examples/kafka.rs"}

`Kafka` is both a loader and a storage backend, so the example builds two of them from one rdkafka `ClientConfig`. `auto.offset.reset` set to `earliest` makes a new consumer group start at the oldest message in the topic.

The loader reads the topic `loader`. The storage writes to the topic `storage`. Before the first node, the pipeline calls `setup` on every storage. The Kafka `setup` returns an error only when its metadata check reports the topic missing. A metadata request for a named topic almost always returns an entry, so `setup` passes and `create_topic_if_not_exists(true)` rarely does anything.

`batch_size` sets how many nodes the storage sends per batch. The default is 32; the example uses 2.

### Kafka pipeline

::snippet{src="@swiftide/examples/kafka.rs" anchor="pipeline" title="examples/kafka.rs"}

Each Kafka message with a UTF-8 payload becomes one node, with the payload as its chunk. The loader skips other messages and logs them only at debug level. `Embed` embeds 10 nodes per call with FastEmbed, a local embedder. The Kafka storage produces one message per node on the storage topic. The pipeline runs until you stop the process.

### Storage keys and payloads

As storage, `Kafka` writes each node with the key `<path>:<node id>` and the node serialized as JavaScript Object Notation (JSON) as the payload. A node from the Kafka loader has an empty path, and its id is derived from the path and the chunk. Two messages with the same payload therefore get the same key.

To change either, set `persist_key_fn` or `persist_payload_fn` on the builder. Both take a plain function pointer, `fn(&TextNode) -> Result<String>`, so a closure that captures variables does not fit.

### Kafka run

Start a broker on `localhost:9092`:

```sh
docker run -d --name kafka -p 9092:9092 apache/kafka:3.9.0
```

To run the unchanged example instead, run it from the root of a swiftide checkout:

```sh
cargo run -p swiftide-examples --example kafka
```

In a second terminal, produce messages on `loader`, one per line:

```sh
docker exec -it kafka /opt/kafka/bin/kafka-console-producer.sh --bootstrap-server localhost:9092 --topic loader
```

Read the embedded nodes back from `storage`:

```sh
docker exec -it kafka /opt/kafka/bin/kafka-console-consumer.sh --bootstrap-server localhost:9092 --topic storage --from-beginning
```

Nothing appears until you have produced 10 messages, the embed batch size.

## Fluvio

Add Swiftide with the `fluvio`, `fastembed` and `qdrant` feature flags. The example also uses `fluvio::Offset`, which Swiftide does not re-export, so add the fluvio crate at the version Swiftide depends on:

```sh
cargo add swiftide@{{swiftide_version}} --features fluvio,fastembed,qdrant
cargo add fluvio@0.50.1 --no-default-features
cargo add tokio --features macros,rt-multi-thread
```

A different fluvio version gives you a different `Offset` type, and the consumer config does not accept it.

### Fluvio loader

::snippet{src="@swiftide/examples/fluvio.rs" anchor="setup" title="examples/fluvio.rs"}

The loader reads partition 0 of the topic `hello-rust`. `Offset::from_end(1)` starts one record before the end of the partition.

`Fluvio` is a loader only; it has no storage side. Without `fluvio_config`, the loader connects with `Fluvio::connect()`, which uses the current profile in `~/.fluvio/config`. Pass a `FluvioConfig` to `fluvio_config` to connect elsewhere. `Fluvio::from_consumer_config` is a shorter constructor when the current profile is enough.

### Fluvio pipeline

::snippet{src="@swiftide/examples/fluvio.rs" anchor="pipeline" title="examples/fluvio.rs"}

Each record becomes a node whose chunk is the record value. The embeddings go to the Qdrant collection `swiftide-examples`. Its vector size of 384 matches the default FastEmbed model. Qdrant is reached over gRPC at `http://localhost:6334` unless `QDRANT_URL` says otherwise.

### Fluvio run

Start Qdrant and a local Fluvio cluster, and create the topic:

```sh
docker run -d -p 6333:6333 -p 6334:6334 qdrant/qdrant
fluvio cluster start
fluvio topic create hello-rust
```

Run the example from a swiftide checkout, then produce records, one per line, in a second terminal:

```sh
cargo run -p swiftide-examples --example fluvio
fluvio produce hello-rust
```

## Gotchas

- **Errors stop the run.** `run` returns at the first error in the stream, such as a failed `recv` or a failed embedding call. To keep a long-running stream alive, put `.log_errors().filter_errors()` directly after `from_loader`. Placed after `then_in_batch` or `then_store_with`, an error drops the nodes already buffered for that batch along with it.
- **Partial batches wait.** Batch transformers and batch storage wait for a full batch. On a quiet topic, the last few nodes wait for new messages.
- **Order is not kept.** The pipeline stores batches concurrently, so nodes can land in the storage topic in a different order than they were read.
- **Metadata is embedded.** The default embed mode embeds the metadata together with the chunk, so the loader's metadata fields become part of the embedded text. [Rich metadata and multiple vectors](/cookbook/rich-metadata-and-multiple-vectors/) shows the other embed modes.
- **Loaders panic when you build the pipeline.** `Pipeline::from_loader` starts the loader, so the Kafka loader panics there when it cannot create the consumer or subscribe, and the Fluvio loader panics there when it cannot connect. For Kafka that means an invalid configuration; an unreachable broker does not panic, and the loader keeps waiting.
- **Fluvio needs a multi-thread runtime.** The Fluvio loader connects inside `tokio::task::block_in_place`, which panics on a current-thread Tokio runtime. `#[tokio::main]` uses a multi-thread runtime by default.
- **FastEmbed downloads its model.** The first `FastEmbed::try_default()` downloads the embedding model from Hugging Face.

## Reference

- [`Kafka`](docsrs:swiftide/integrations/kafka/struct.Kafka.html)
- [`KafkaBuilder`](docsrs:swiftide/integrations/kafka/struct.KafkaBuilder.html)
- [`ClientConfig`](https://docs.rs/rdkafka/0.39.0/rdkafka/config/struct.ClientConfig.html) (rdkafka)
- [`Fluvio`](docsrs:swiftide/integrations/fluvio/struct.Fluvio.html)
- [`FluvioBuilder`](docsrs:swiftide/integrations/fluvio/struct.FluvioBuilder.html)
- [`ConsumerConfigExt`](https://docs.rs/fluvio/0.50.1/fluvio/consumer/struct.ConsumerConfigExt.html) (fluvio)
- [`Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html)
- [`FastEmbed`](docsrs:swiftide/integrations/fastembed/struct.FastEmbed.html)
- [`Qdrant`](docsrs:swiftide/integrations/qdrant/struct.Qdrant.html)
- [`Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html)
- [`Loader`](docsrs:swiftide/traits/trait.Loader.html)
- [`Persist`](docsrs:swiftide/traits/trait.Persist.html)

## Next

[Other providers and stores](/cookbook/providers-and-stores/)
