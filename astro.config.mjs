import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

// https://astro.build/config
export default defineConfig({
  site: "https://swiftide.bosun.ai",
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
      ],
      logo: { src: "./src/assets/logo.png" },
      social: {
        github: "https://github.com/bosun-ai/swiftide",
      },
      sidebar: [
        {
          label: "Guides",
          items: [
            // Each item here is one entry in the navigation menu.
            { label: "Example Guide", link: "/guides/example/" },
          ],
        },
        {
          label: "Reference",
          autogenerate: { directory: "reference" },
        },
      ],
    }),
  ],
});
