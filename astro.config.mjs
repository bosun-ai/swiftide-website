import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

import tailwind from "@astrojs/tailwind";
import starlightLinksValidatorPlugin from "starlight-links-validator";

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
      description:
        "Blazing fast data pipelines for Retrieval Augmented Generation written in Rust",
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
          label: "In depth",
          autogenerate: {
            directory: "in-depth",
          },
        },
        {
          label: "Examples",
          autogenerate: {
            directory: "examples",
          },
        },

        {
          label: "Reference",
          link: "https://docs.rs/swiftide/latest/swiftide/",
          badge: {
            variant: "note",
            text: "docs.rs",
          },

          attrs: { target: "_blank" },
        },
        {
          label: "Troubleshooting",
          link: "/troubleshooting/",
        },
      ],
    }),
    tailwind({ applyBaseStyles: false }),
  ],
});
