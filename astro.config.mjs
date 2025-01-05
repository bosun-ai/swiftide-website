import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import tailwind from "@astrojs/tailwind";
import starlightLinksValidatorPlugin from "starlight-links-validator";

import partytown from "@astrojs/partytown";

// https://astro.build/config
export default defineConfig({
  site: "https://swiftide.rs",
  image: {
    service: {
      entrypoint: "astro/assets/services/sharp",
      config: {
        limitInputPixels: false,
      },
    },
  },
  prefetch: {
    prefetchAll: true,
  },
  integrations: [
    starlight({
      title: "swiftide",
      // <script defer data-domain="swiftide.rs" src="https://plausible.io/js/script.js"></script>
      head: [
        {
          tag: "script",
          attrs: {
            defer: true,
            type: "text/partytown",
            "data-domain": "swiftide.rs",
            src: "https://plausible.io/js/script.js",
          },
        },
      ],
      description:
        "Swiftide is a Rust library for building LLM applications. Large language models are amazing, but need context to solve real problems. Swiftide allows you to ingest, transform and index large amounts of data fast, and then query that data so it it can be injected into prompts. This process is called Retrieval Augmented Generation.",
      editLink: {
        baseUrl: "https://github.com/bosun-ai/swiftide-website/edit/master",
      },
      tableOfContents: {
        minHeadingLevel: 2,
      },
      customCss: [
        // Fontsource files for to regular and semi-bold font weights.
        "@fontsource/fira-code/400.css",
        "@fontsource/fira-code/600.css",
        "./src/styles/custom.css",
        "./src/tailwind.css",
      ],
      favicon: "favicon32.png",
      plugins: [starlightLinksValidatorPlugin()],
      logo: {
        src: "./src/assets/logo.png",
      },
      social: {
        github: "https://github.com/bosun-ai/swiftide",
        linkedin: "https://www.linkedin.com/company/bosun-ai/",
        discord: "https://discord.gg/3jjXYen9UY",
      },
      sidebar: [
        {
          label: "What is swiftide?",
          link: "/what-is-swiftide/",
        },
        {
          label: "Getting Started",
          autogenerate: {
            directory: "getting-started",
          },
        },
        {
          label: "Core Concepts",
          autogenerate: {
            directory: "core-concepts",
          },
        },
        {
          label: "Indexing Your Data",
          collapsed: true,
          autogenerate: {
            directory: "indexing",
          },
        },
        {
          label: "Querying With Your Data",
          collapsed: true,
          autogenerate: {
            directory: "querying",
          },
        },
        {
          label: "Agents",
          collapsed: true,
          autogenerate: {
            directory: "agents",
          },
        },
        {
          label: "In depth",
          collapsed: true,
          autogenerate: {
            directory: "in-depth",
          },
        },
        {
          label: "Tutorials",
          collapsed: true,
          autogenerate: {
            directory: "tutorials",
          },
        },
        {
          label: "Reference",
          link: "https://docs.rs/swiftide/latest/swiftide/",
          badge: {
            variant: "note",
            text: "docs.rs",
          },
          attrs: {
            target: "_blank",
          },
        },
        {
          label: "Troubleshooting",
          link: "/troubleshooting/",
        },
      ],
    }),
    tailwind({
      applyBaseStyles: false,
    }),
    partytown(),
  ],
});
