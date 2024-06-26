import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

import tailwind from "@astrojs/tailwind";

// https://astro.build/config
export default defineConfig({
  site: "https://swiftide.rs",
  integrations: [
    starlight({
      title: "swiftide",
      editLink: {
        baseUrl: "https://github.com/bosun-ai/swiftide-website/edit/master",
      },
      customCss: [
        // Fontsource files for to regular and semi-bold font weights.
        "@fontsource/fira-code/400.css",
        "@fontsource/fira-code/600.css",
        "./src/styles/custom.css",
        "./src/tailwind.css",
      ],
      favicon: "./src/assets/swiftide-favicon32.png",
      logo: {
        src: "./src/assets/logo.png",
      },
      social: {
        github: "https://github.com/bosun-ai/swiftide",
      },
      sidebar: [
        {
          label: "Introduction",
          link: "/introduction/",
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
            directory: "concepts",
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
