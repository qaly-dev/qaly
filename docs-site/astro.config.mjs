import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  integrations: [
    starlight({
      title: 'Qaly',
      description: 'The mobile testing tool built for AI agents.',
      social: {
        github: 'https://github.com/juanpaternina/qaly',
      },
      editLink: {
        baseUrl: 'https://github.com/juanpaternina/qaly/edit/main/docs-site/',
      },
      customCss: ['./src/styles/custom.css'],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'What is Qaly?', link: '/' },
            { label: 'MCP Setup', link: '/getting-started/mcp/' },
            { label: 'CLI Setup', link: '/getting-started/cli/' },
          ],
        },
        {
          label: 'Guides',
          items: [
            { label: 'Recording your first test', link: '/guides/recording/' },
            { label: 'Replay & CI integration', link: '/guides/replay/' },
            { label: 'Automatic self-healing', link: '/guides/self-healing/' },
            { label: 'Adding testIDs', link: '/guides/testids/' },
            { label: 'Framework compatibility', link: '/guides/frameworks/' },
          ],
        },
        {
          label: 'Examples',
          items: [
            { label: 'Create an alarm (Clock)', link: '/examples/alarm/' },
            { label: 'E-commerce checkout', link: '/examples/checkout/' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'MCP tools', link: '/reference/mcp-tools/' },
            { label: 'qaly CLI', link: '/reference/cli/' },
            { label: '.test file format', link: '/reference/test-format/' },
            { label: 'Selector syntax', link: '/reference/selectors/' },
            { label: 'Environment variables', link: '/reference/env-vars/' },
          ],
        },
        { label: 'Benchmarks', link: '/benchmarks/' },
      ],
    }),
  ],
  output: 'static',
});
