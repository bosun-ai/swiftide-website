import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

// https://astro.build/config
export default defineConfig({
  integrations: [
    starlight({
      title: "Swiftide",
      editLink: {
        baseUrl: "https://github.com/bosun-ai/swiftide-website/edit/master",
      },
      // site: "https://swiftide.bosun.ai",
      // integrations: [starlight({ title: "Swiftide" })],
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
