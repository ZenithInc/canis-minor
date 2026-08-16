import { defineConfig } from "astro/config";
import sitemap from "@astrojs/sitemap";
import starlight from "@astrojs/starlight";

export default defineConfig({
  site: "https://zenithinc.github.io",
  base: "/canis-minor",
  trailingSlash: "always",
  integrations: [
    starlight({
      title: "Canis Minor",
      description:
        "An encrypted IPv4 transparent proxy tunnel for Ubuntu 24.04 on x86_64.",
      favicon: "/favicon.svg",
      locales: {
        root: { label: "English", lang: "en" },
        "zh-cn": { label: "简体中文", lang: "zh-CN" },
      },
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/ZenithInc/canis-minor",
        },
      ],
      editLink: {
        baseUrl: "https://github.com/ZenithInc/canis-minor/edit/main/website/",
      },
      lastUpdated: true,
      pagefind: true,
      credits: true,
      disable404Route: true,
      customCss: ["./src/styles/custom.css"],
      head: [
        {
          tag: "meta",
          attrs: { name: "theme-color", content: "#07101f" },
        },
      ],
      sidebar: [
        {
          label: "Getting started",
          translations: { "zh-CN": "入门" },
          items: [
            { slug: "getting-started" },
            { slug: "getting-started/releases" },
            { slug: "getting-started/source" },
          ],
        },
        {
          label: "Concepts",
          translations: { "zh-CN": "核心概念" },
          items: [
            { slug: "concepts/architecture" },
            { slug: "concepts/traffic-boundaries" },
          ],
        },
        {
          label: "Reference",
          translations: { "zh-CN": "参考" },
          items: [{ slug: "reference/configuration" }],
        },
        {
          label: "Operations",
          translations: { "zh-CN": "运维" },
          items: [
            { slug: "operations/deployment" },
            { slug: "operations/troubleshooting" },
            { slug: "operations/uninstall" },
            { slug: "operations/verification" },
          ],
        },
        {
          label: "Project",
          translations: { "zh-CN": "项目" },
          items: [
            { slug: "project/acceptance" },
            { slug: "project/contributing" },
            { slug: "project/roadmap" },
          ],
        },
      ],
    }),
    sitemap(),
  ],
});
