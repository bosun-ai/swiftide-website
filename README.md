# swiftide.rs

Source of [swiftide.rs](https://swiftide.rs), the documentation site for [Swiftide](https://github.com/bosun-ai/swiftide), a Rust library for building LLM applications.

The site is built with [Astro](https://astro.build) and [Starlight](https://starlight.astro.build). Every line of Rust on it is compiled: pages never contain hand-written Rust, they excerpt it from files that `cargo` checks.

## Development

Requirements: Node 24, pnpm 10 (`corepack enable`), git, and rustup for the snippet crate.

```sh
scripts/setup.sh   # submodule + tags, pnpm install, Rust toolchain
pnpm dev           # http://localhost:4321
pnpm verify        # everything CI checks; run before pushing
```

| Command | What it does |
| --- | --- |
| `pnpm dev` | Dev server. Changes to excerpted files in `vendor/swiftide/examples`, `snippets/src`, `Cargo.toml` and `src/data/integrations.yaml` refresh the affected pages. |
| `pnpm build` | Production build into `dist/` (`astro build --force`, so no stale content cache). |
| `pnpm test` | Unit tests for the directive expander, snippet extraction and lints (vitest). |
| `pnpm lint:idents [page.md …]` | Content lint: unknown swiftide identifiers in inline code, hand-written ```` ```rust ```` blocks, unresolvable `::snippet`s. |
| `pnpm check:snippets` | `cargo check` of the `snippets/` crate against the pinned swiftide. |
| `pnpm verify:dist` | Checks `dist/`: `.md` sibling per page, no unexpanded directives, llms files, old URLs still resolve. |
| `pnpm verify` | All of the above plus `astro check`. |

## Layout

```
src/content/docs/      pages (.md; .mdx only where components are needed)
src/data/              integrations.yaml (the integrations matrix)
src/lib/               swiftide version context, snippet extraction, remark plugin, md renderer
src/sidebar.ts         navigation
src/redirects.ts       old URLs -> new pages
snippets/              swiftide-docs-snippets: Rust excerpts that are not part of a swiftide example
vendor/swiftide/       swiftide itself, as a git submodule (the pinned version)
scripts/               lints, dist verification, setup and bump scripts
tests/                 vitest tests and fixtures
```

## Writing pages

Pages are markdown. On top of Starlight's syntax the site supports these directives, expanded at build time by `src/lib/remark/remark-swiftide.ts` (for the HTML page and for the `.md` copy that LLMs read):

```md
::snippet{src="@swiftide/examples/hello_agents.rs" anchor="build_agent" title="examples/hello_agents.rs" collapse="1-4" mark="8-10"}
::snippet{src="@snippets/src/start/installation.rs" anchor="openai_client"}
::include{src="@swiftide/CHANGELOG.md" strip="^# Changelog" shift="1"}
::feature-flags
::integrations{kind="storage"}

Run `cargo add swiftide@{{swiftide_version}}`. Read [the source](https://github.com/bosun-ai/swiftide/tree/{{swiftide_sha}}/examples). See [`Agent`](docsrs:swiftide/agents/struct.Agent.html). :unreleased
```

- **`::snippet`** cuts a region out of a compiled file. `@swiftide/` points into `vendor/swiftide`, `@snippets/` into `snippets/`. Select the region with `anchor="name"`, which matches `// ANCHOR: name` and `// ANCHOR_END: name` markers in the source (nested markers are stripped, the excerpt is dedented), or temporarily with `lines="20-48"`. Optional: `title`, `collapse`, `mark`, `ins`, `del` (line ranges relative to the excerpt), `frame`, `wrap`, `lang`, and `source="false"` to drop the "Source:" permalink that is added below every snippet.
- **`::include`** splices a markdown file in (`strip` drops matching lines, `shift` demotes headings, `llms="false"` keeps it out of the llms files). **`::feature-flags`** renders the feature flag tables from swiftide's `Cargo.toml`. **`::integrations`** renders the integrations matrix from `src/data/integrations.yaml`, optionally filtered by `kind`.
- **`{{swiftide_version}}`** is the pinned release (e.g. `0.33.0`) and **`{{swiftide_sha}}`** the full pinned commit, for GitHub URLs, `git checkout` and `cargo add --rev`; **`docsrs:`** links point at docs.rs for that release; **`:unreleased`** marks APIs that are on swiftide master but not released yet. Frontmatter `swiftide: { since: "0.32", unreleased: true }` adds the same badges next to the page title. In `.mdx` pages `{` starts a JSX expression, so the `{{…}}` placeholders only work in `.md` pages.

Anything that cannot be resolved (missing file or anchor, unknown attribute, out-of-range `mark`, unknown placeholder) fails the build with the page and line.

To show new Rust code, add it to `snippets/src/<section>/<topic>.rs` inside a function that compiles, wrap the part to show in anchors, and run `pnpm check:snippets`. Code from swiftide's own examples needs the anchors added in the swiftide repository.

## The swiftide pin

`vendor/swiftide` is a git submodule. Its recorded commit (the gitlink) is the single source of the version the site documents: `src/lib/swiftide.ts` derives the release (`git describe --tags`), the docs.rs URLs and the GitHub permalinks from it, and the build fails if the checkout does not match the pin or the tags are missing.

To move the pin:

```sh
scripts/bump-swiftide.sh                 # to origin/master, if swiftide CI is green for it
scripts/bump-swiftide.sh v0.34.0         # or any ref / SHA
git commit -m "chore: pin swiftide $(git -C vendor/swiftide describe --tags)"
```

The script refuses commits whose swiftide CI is not green, stages the new gitlink, updates `snippets/Cargo.lock` and runs `pnpm verify`. The `Bump swiftide` workflow does the same and opens a PR; swiftide's CI can trigger it with a `swiftide-master-green` repository dispatch.

## For LLMs and coding agents

Every page is also served as markdown at `<page>.md` (linked with `<link rel="alternate" type="text/markdown">`), with all snippets expanded. `/llms.txt`, `/llms-full.txt`, `/llms-small.txt` and per-section sets are generated on build. Page actions (Copy / View as Markdown, Open in ChatGPT / Claude) sit next to every title.

## Deployment

`.github/workflows/site.yml` checks the snippets and builds and verifies the site on every push and PR, and deploys `master` to GitHub Pages.
