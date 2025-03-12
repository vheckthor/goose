import { themes as prismThemes } from "prism-react-renderer";
import type { Config } from "@docusaurus/types";
import type * as Preset from "@docusaurus/preset-classic";

const config: Config = {
  title: "Goose Prompt Library",
  tagline: "A curated collection of prompts and goosehints for Goose",
  favicon: "img/favicon.ico",

  // Set the production url of your site here
  url: "https://block.github.io",
  baseUrl: "/goose/v1/prompts/",

  organizationName: "block",
  projectName: "goose",

  onBrokenLinks: "throw",
  onBrokenMarkdownLinks: "warn",

  i18n: {
    defaultLocale: "en",
    locales: ["en"],
  },

  presets: [
    [
      "classic",
      {
        docs: {
          routeBasePath: "/", // Serve the docs at the site's root
          sidebarPath: "./sidebars.ts",
          // Remove this to remove the "edit this page" links.
          editUrl: "https://github.com/block/goose/tree/main/documentation/prompt-library",
        },
        blog: false,
        theme: {
          customCss: "./src/css/custom.css",
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    // Replace with your project's social card
    image: "img/social-card.png",
    navbar: {
      title: "Goose Prompt Library",
      logo: {
        alt: "Goose Logo",
        src: "img/logo.png",
      },
      items: [
        {
          href: "https://block.github.io/goose/docs/quickstart",
          label: "Docs",
          position: "right",
        },
        {
          href: "https://block.github.io/goose/v1/extensions",
          label: "Extensions",
          position: "right",
        },
        {
          href: "https://github.com/block/goose",
          label: "GitHub",
          position: "right",
        },
      ],
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "Community",
          items: [
            {
              label: "Discord",
              href: "https://discord.gg/block-opensource",
            },
            {
              label: "Twitter",
              href: "https://twitter.com/blockopensource",
            },
          ],
        },
        {
          title: "More",
          items: [
            {
              label: "GitHub",
              href: "https://github.com/block/goose",
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Block, Inc.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;