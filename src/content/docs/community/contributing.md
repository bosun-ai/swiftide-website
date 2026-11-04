---
title: "Contributing and community"
description: "Ask questions on Discord, file issues from the bug report and feature request templates, and open pull requests that pass Swiftide's CI checks."
sidebar:
  order: 1
---

By the end of this page you know where to ask for help, how to file an issue, and which checks a pull request must pass before review.

## Help and discussion

- [Discord](https://discord.gg/3jjXYen9UY) gives you the fastest feedback. Release changelogs are posted there automatically when a GitHub release is published.
- [GitHub issues](https://github.com/bosun-ai/swiftide/issues) are the place for design discussion.
- The [Bosun blog](https://blog.bosun.ai/) has background posts on tasks, streaming agents, human-in-the-loop flows and Rust performance for artificial intelligence (AI) tools.

Swiftide is pre-1.0. The application programming interface (API) can change between releases. The [API docs](docsrs:swiftide/index.html) are the most reliable source for exact signatures. The [changelog](/reference/changelog/) lists what changed in each release.

## Issues

The repository has two issue templates:

- [Bug report](https://github.com/bosun-ai/swiftide/issues/new?template=bug_report.md): describe the bug, give a minimal example that reproduces it, and state the behavior you expected.
- [Feature request](https://github.com/bosun-ai/swiftide/issues/new?template=feature_request.md): describe the problem, the solution you want, and the alternatives you considered.

To suggest an enhancement without writing the code, open a feature request instead of a pull request. To find something to work on, browse the [open issues](https://github.com/bosun-ai/swiftide/issues).

## Design principles

- Write simple, thin wrappers with sane defaults.
- Offer a builder (`derive_builder`) for customization.
- Keep Rust complexity such as `Arc`, `Box`, lifetimes and pinning away from library users.
- Follow the [Rust API Guidelines on naming](https://rust-lang.github.io/api-guidelines/naming.html).
- Watch allocations in indexing and querying. Both are performance-sensitive.

The library crates opt in to the workspace lints. These lints forbid `unsafe` code and set Clippy's `pedantic` and `cargo` lint groups to warn. Continuous integration (CI) turns warnings into errors.

## Development setup

- **Rust toolchain.** `rust-toolchain.toml` pins Rust 1.97.1, so `rustup` installs the right version for you.
- **Nightly rustfmt.** Formatting needs nightly because `rustfmt.toml` wraps comments. Install it with `rustup component add rustfmt --toolchain nightly`.
- **Cargo tools.** The checks use tools that do not ship with `rustup`. Install them with `cargo install cargo-hack cargo-deny cargo-insta typos-cli`.
- **protoc and libcurl.** CI installs the Protocol Buffers compiler and the libcurl development headers before it builds with every feature flag. Install both locally if you run `--all-features` builds.
- **Docker.** The integration tests start their services with testcontainers. Shared fixtures and container helpers live in `swiftide-test-utils/`.
- **Secrets.** Keep credentials in environment variables or `mise`. Never commit them.

## Checks

These commands mirror the main CI jobs. Run them before you request a review:

```sh
cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -j 2 --tests --all-features --no-fail-fast
cargo test --doc --all-features --no-fail-fast
cargo hack check --each-feature --no-dev-deps
typos
cargo deny check
```

`cargo hack` checks each [feature flag](/reference/feature-flags/) on its own. `cargo deny check` checks advisories, licenses, banned crates and crate sources against `deny.toml`.

CI also checks `swiftide-agents-contrib` once per capability, without default features, and builds its rustdoc with warnings as errors. If you change that crate, run the same steps for the capability you touched, for example `planning`:

```sh
cargo check -p swiftide-agents-contrib --lib --no-default-features --features planning
cargo test -p swiftide-agents-contrib --no-default-features --features planning
RUSTDOCFLAGS="-D warnings" cargo doc -p swiftide-agents-contrib --no-default-features --features planning --no-deps
cargo check -p swiftide --lib --no-default-features --features agents-contrib-planning
```

The other capabilities are `context-management`, `file-tools` and `lazy-executor`, with the `swiftide` feature flags `agents-contrib-context`, `agents-contrib-file-tools` and `agents-contrib-lazy-executor`.

While you iterate, run focused checks on the crates you changed:

```sh
cargo check -p swiftide-agents
cargo test -p swiftide-integrations
```

- `RUST_LOG=info` turns on verbose diagnostics.
- Some tests are marked `#[ignore]`, and CI skips them. Most download HuggingFace models through the `fastembed` feature flag. One calls AWS Bedrock and bills your account. Run them only on purpose, with the feature flags they need, for example `cargo test -p swiftide-integrations --features fastembed -- --ignored`.
- When an insta snapshot is new or no longer matches, the test writes a pending `.snap.new` file. Review it with `cargo insta review`, and commit only after it clears the pending files.
- Benchmarks run with `cargo bench -p benchmarks` on every push to master. The job fails and comments on the commit when a benchmark regresses past the 200% alert threshold.

## Commits and pull requests

Commits and pull request (PR) titles follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). Write a lowercase imperative summary with an optional scope, such as `feat(agents): ...` or `fix(indexing): ...`.

- CI checks every PR title against the format.
- A `!` before the colon, as in `feat(agents)!:`, marks a breaking change. CI adds the `breaking change` label when the PR title has this marker. The check only accepts a one-word scope, so a title such as `feat(agents-contrib)!:` gets no label.
- A PR labeled `do not merge` fails CI on purpose.

To open a PR, fork the repository, create a feature branch, commit, push, and open the PR against `master`. The description covers:

- what changed and why
- the linked GitHub issue, if there is one
- any API or schema impact
- before and after traces or logs when behavior changes

Functional changes come with tests, and with benchmarks where performance matters.

## Documentation

- Document every public function with rustdoc.
- Update the user documentation in [bosun-ai/swiftide-website](https://github.com/bosun-ai/swiftide-website). Each documentation page has an edit link.
- The site compiles every line of Rust it shows. Pages excerpt code from source files and never contain hand-written Rust. The website `README.md` explains the snippet directives.
- To show code from `examples/` in the swiftide repository, wrap the region in `// ANCHOR: name` and `// ANCHOR_END: name` there first. A page can use the anchor once the website pins a swiftide commit that contains it.

## AI-generated code

Code written with an AI assistant is welcome. It is reviewed like any other code, so read and test what you submit. Coding agents should read `AGENTS.md` in the swiftide repository for the workspace layout, commands and expectations.

## Core team

- [timonv](https://github.com/timonv)
- [tinco](https://github.com/tinco)

Swiftide is part of the [bosun.ai](https://bosun.ai) project. The full list of contributors is on the [contributors graph](https://github.com/bosun-ai/swiftide/graphs/contributors).

## License

Swiftide is distributed under the MIT License, copyright 2024 Bosun.ai.

## Reference

- [swiftide API docs](docsrs:swiftide/index.html)

## Next

[What is Swiftide](/start/what-is-swiftide/)
