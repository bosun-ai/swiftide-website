// @ts-check
import { unified } from '@astrojs/markdown-remark';
import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';
import linksValidator from 'starlight-links-validator';
import llmActions from 'starlight-llm-actions';
import llmsTxt from 'starlight-llms-txt';
import remarkSwiftide from './src/lib/remark/remark-swiftide.ts';
import { snippetWatch } from './src/lib/snippet-watch.ts';
import { loadSwiftideContext } from './src/lib/swiftide.ts';
import { redirects } from './src/redirects.ts';
import { sidebar } from './src/sidebar.ts';

const projectRoot = process.cwd();
const swiftide = loadSwiftideContext(projectRoot);
const pinned = swiftide.commitsAhead > 0 ? `${swiftide.tag} + ${swiftide.commitsAhead} commits, ${swiftide.shortSha}` : swiftide.tag;

export default defineConfig({
	site: 'https://swiftide.rs',
	trailingSlash: 'ignore',
	markdown: {
		// Astro 7 renders markdown with Sätteri by default; remark plugins need the unified processor.
		processor: unified({ remarkPlugins: [[remarkSwiftide, { projectRoot, swiftide, target: 'html' }]] }),
	},
	redirects,
	integrations: [
		snippetWatch(projectRoot),
		starlight({
			title: 'swiftide',
			description:
				'Swiftide is a Rust library for building LLM applications: composable agents, typed task graphs and streaming indexing and query pipelines.',
			logo: { src: './src/assets/mark.svg', replacesTitle: false },
			favicon: '/favicon.svg',
			editLink: { baseUrl: 'https://github.com/bosun-ai/swiftide-website/edit/master/' },
			lastUpdated: true,
			tableOfContents: { minHeadingLevel: 2, maxHeadingLevel: 3 },
			customCss: [
				'./src/styles/fonts.css',
				'./src/styles/tokens.css',
				'./src/styles/starlight.css',
				'./src/styles/prose.css',
				'./src/styles/code.css',
				'./src/styles/home.css',
			],
			head: [
				{ tag: 'link', attrs: { rel: 'icon', href: '/favicon32.png', sizes: '32x32', type: 'image/png' } },
				{ tag: 'meta', attrs: { property: 'og:image', content: 'https://swiftide.rs/og.png' } },
				{ tag: 'meta', attrs: { property: 'og:image:width', content: '1200' } },
				{ tag: 'meta', attrs: { property: 'og:image:height', content: '630' } },
				{ tag: 'meta', attrs: { name: 'twitter:card', content: 'summary_large_image' } },
				{ tag: 'meta', attrs: { name: 'twitter:image', content: 'https://swiftide.rs/og.png' } },
				{ tag: 'meta', attrs: { name: 'theme-color', content: '#0e1013' } },
				{
					tag: 'script',
					attrs: { defer: true, 'data-domain': 'swiftide.rs', src: 'https://plausible.io/js/script.js' },
				},
			],
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/bosun-ai/swiftide' },
				{ icon: 'discord', label: 'Discord', href: 'https://discord.gg/3jjXYen9UY' },
			],
			components: {
				Hero: './src/components/overrides/Hero.astro',
				PageTitle: './src/components/overrides/PageTitle.astro',
				SiteTitle: './src/components/overrides/SiteTitle.astro',
			},
			sidebar: sidebar(swiftide),
			plugins: [
				linksValidator({ errorOnLocalLinks: true, exclude: ['/llms*.txt', '/**/*.md'] }),
				llmActions({
					renderMarkdown: { module: './src/lib/md-renderer.ts' },
					linkAlternate: { absolute: true },
					llmsTxt: false,
					actions: {
						copyMarkdown: true,
						viewMarkdown: true,
						openIn: { providers: { chatgpt: true, claude: true, gemini: false, copilot: false, perplexity: false } },
					},
				}),
				llmsTxt({
					projectName: 'Swiftide',
					description:
						'Swiftide is a Rust library for building LLM applications: composable agents with tools and hooks, typed task graphs, and streaming indexing and query pipelines.',
					details: [
						`These docs describe swiftide ${swiftide.version} (pinned at ${pinned}).`,
						`Pin the crate with \`cargo add swiftide@${swiftide.version}\`; features are opt-in through Cargo feature flags.`,
						`API reference: ${swiftide.docsRsBase}. Append \`.md\` to any page URL for its markdown, e.g. https://swiftide.rs/start/installation.md.`,
					].join(' '),
					promote: ['index*', 'start/**'],
					demote: ['cookbook/**'],
					// `exclude` only applies to llms-small.txt: it keeps the start pages and core guides.
					// Recipes, showcases, integrations and reference stay in llms-full.txt and the custom sets.
					exclude: [
						'404',
						'community/**',
						'cookbook/**',
						'integrations/**',
						'observability/**',
						'reference/**',
						'showcases/**',
					],
					customSets: [
						{ label: 'Agents', paths: ['agents/**'], description: 'building agents with tools, hooks and executors' },
						{ label: 'Tasks', paths: ['tasks/**'], description: 'typed task graphs with fan-out, join and pause/resume' },
						{
							label: 'Indexing and querying',
							paths: ['indexing/**', 'querying/**'],
							description: 'streaming indexing pipelines and query pipelines (RAG)',
						},
						{ label: 'Integrations', paths: ['integrations/**'], description: 'model providers, vector stores and other integrations' },
						{ label: 'Showcases', paths: ['showcases/**'], description: 'complete example programs and what they produce' },
					],
					// Expressive Code's "N collapsed lines" toggle is chrome, not code; `.sw-llms-exclude`
					// marks content kept out of every llms file (`::include{llms="false"}`).
					// The small file also drops heading anchors and snippet "Source:" permalinks.
					// Keep headings and list items on their own lines in llms-small.txt; it is read by models, not bytes-counted.
					minify: { whitespace: false },
					customSelectors: {
						all: ['summary:has(.expand)', '.sw-llms-exclude'],
						small: ['.sl-anchor-link', '.sw-source'],
					},
				}),
			],
		}),
	],
});
