import type { Code, Root } from 'mdast';
import remarkDirective from 'remark-directive';
import remarkGfm from 'remark-gfm';
import remarkParse from 'remark-parse';
import { unified } from 'unified';
import { VFile } from 'vfile';
import { describe, expect, it } from 'vitest';
import { renderPageMarkdown, versionNote } from '../src/lib/md-renderer.ts';
import remarkSwiftide from '../src/lib/remark/remark-swiftide.ts';
import { FIXTURE_ROOT, fixtureSwiftide } from './helpers.ts';

const options = { projectRoot: FIXTURE_ROOT, swiftide: fixtureSwiftide };

/** Runs the html-target pipeline (what Astro runs) and returns the transformed tree. */
function html(markdown: string): Root {
	const processor = unified()
		.use(remarkParse)
		.use(remarkGfm)
		.use(remarkDirective)
		.use(remarkSwiftide, { ...options, target: 'html' });
	const file = new VFile({ path: 'page.md', value: markdown });
	return processor.runSync(processor.parse(file), file) as Root;
}

const markdown = (source: string, mdx = false) => renderPageMarkdown(source, { path: 'page.md', mdx, options });

describe('::snippet', () => {
	it('emits a code block with Expressive Code meta and a source permalink', () => {
		const tree = html('::snippet{src="@snippets/src/agents.rs" anchor="outer" title="agents.rs" collapse="1" mark="2-3"}');
		const [code, source] = tree.children;
		expect(code).toMatchObject({ type: 'code', lang: 'rust', meta: 'title="agents.rs" collapse={1} {2-3}' });
		expect((code as Code).value).not.toContain('ANCHOR');
		expect(source).toMatchObject({
			type: 'paragraph',
			data: { hProperties: { className: ['sw-source'] } },
			children: [
				{ type: 'text', value: 'Source: ' },
				{
					type: 'link',
					url: 'https://github.com/bosun-ai/swiftide-website/blob/master/snippets/src/agents.rs#L4-L10',
					children: [{ type: 'inlineCode', value: 'snippets/src/agents.rs#L4-L10' }],
				},
			],
		});
	});

	it('links swiftide sources to the pinned sha', () => {
		const tree = html('::snippet{src="@swiftide/examples/hello.rs" anchor="build"}');
		expect(JSON.stringify(tree.children[1])).toContain(`${fixtureSwiftide.ghBlob}examples/hello.rs#L7-L7`);
	});

	it('can drop the source line', () => {
		expect(html('::snippet{src="@swiftide/examples/hello.rs" anchor="build" source="false"}').children).toHaveLength(1);
	});

	it('keeps only the title in markdown', async () => {
		const out = await markdown('Intro\n\n::snippet{src="@swiftide/examples/hello.rs" anchor="build" title="hello.rs" mark="1"}\n');
		expect(out).toBe(
			[
				'Intro',
				'',
				'```rust title="hello.rs"',
				'let agent = Agent::builder().build();',
				'```',
				'',
				`Source: [\`examples/hello.rs#L7-L7\`](${fixtureSwiftide.ghBlob}examples/hello.rs#L7-L7)`,
			].join('\n'),
		);
	});

	it('fails at the directive position with the reason', () => {
		expect(() => html('# Title\n\n::snippet{src="@snippets/src/agents.rs" anchor="missing"}')).toThrow(
			/swiftide ::snippet: Anchor "missing" not found/,
		);
		try {
			html('# Title\n\n::snippet{src="@snippets/src/agents.rs" anchor="missing"}');
		} catch (error) {
			expect(error).toMatchObject({ file: 'page.md', line: 3, column: 1 });
		}
	});

	it('rejects unknown attributes and missing src', () => {
		expect(() => html('::snippet{src="@snippets/src/agents.rs" anchr="outer"}')).toThrow(/Unknown attribute "anchr"/);
		expect(() => html('::snippet{anchor="outer"}')).toThrow(/needs a src/);
	});

	it('rejects ranges outside the excerpt', () => {
		expect(() => html('::snippet{src="@snippets/src/agents.rs" anchor="inner" mark="2"}')).toThrow(/outside the excerpt/);
	});
});

describe('other directives', () => {
	it('fails on unknown block directives', () => {
		expect(() => html('::snipet{src="x"}')).toThrow(/unknown block directive/);
	});

	it('leaves container directives (asides) alone', async () => {
		expect(await markdown(':::note\nHello\n:::')).toBe(':::note\nHello\n:::');
	});

	it('includes markdown with strip and shift', () => {
		const tree = html('::include{src="@swiftide/CHANGELOG.md" strip="^# Changelog" shift="1"}');
		const headings = tree.children.filter((n) => n.type === 'heading').map((n) => ('depth' in n ? n.depth : 0));
		expect(headings).toEqual([3, 4, 3]);
		expect(JSON.stringify(tree)).not.toContain('"Changelog"');
	});

	it('can keep included content out of the llms files', async () => {
		const [wrapper] = html('::include{src="@swiftide/CHANGELOG.md" llms="false"}').children;
		expect(wrapper).toMatchObject({ data: { hName: 'div', hProperties: { className: ['sw-llms-exclude'] } } });
		expect(await markdown('::include{src="@swiftide/CHANGELOG.md" llms="false"}')).toContain('## [0.2.0]');
	});

	it('renders the feature flag tables', async () => {
		const out = await markdown('::feature-flags');
		expect(out).toContain('## General');
		expect(out).toContain('## Integrations');
		expect(out).toMatch(/\| `openai` +\| OpenAI with `Responses` support across two lines \|/);
		expect(out).toMatch(/\| `anthropic` +\| Undocumented/);
		expect(out).toMatch(/\| `mcp` .*\| `swiftide-agents` +\|/);
	});

	it('renders the integrations matrix and filters by kind', async () => {
		const all = await markdown('::integrations');
		expect(all).toContain('| Integration');
		expect(all).toContain('[`OpenAI`](https://docs.rs/swiftide/0.33.0/swiftide/integrations/openai/type.OpenAI.html)');
		expect(all).toContain(`[\`hello.rs\`](${fixtureSwiftide.ghBlob}examples/hello.rs)`);
		expect(all).toContain('[integrations/openai](/integrations/openai/)');
		expect(all).toContain('LLM, Embeddings');
		const storage = await markdown('::integrations{kind="storage"}');
		expect(storage).toContain('Qdrant');
		expect(storage).not.toContain('OpenAI');
		expect(() => html('::integrations{kind="magic"}')).toThrow(/kind="magic" must be one of/);
	});

	it('renders :unreleased per target', async () => {
		expect(await markdown('New API :unreleased')).toBe('New API (unreleased: on swiftide master, not yet in 0.33.0)');
		const tree = html('New API :unreleased');
		expect(JSON.stringify(tree)).toContain('"sw-badge--unreleased"');
	});

	it('rejects block directives used inline', () => {
		expect(() => html('See :snippet{src="x"}')).toThrow(/used inline/);
	});
});

describe('inline rewriting', () => {
	it('substitutes the version everywhere authors write it', async () => {
		const out = await markdown(
			'Run `cargo add swiftide@{{swiftide_version}}`, version {{ swiftide_version }}.\n\n```sh\ncargo add swiftide@{{swiftide_version}}\n```',
		);
		expect(out).toBe(
			'Run `cargo add swiftide@0.33.0`, version 0.33.0.\n\n```sh\ncargo add swiftide@0.33.0\n```',
		);
	});

	it('rejects unknown placeholders but leaves template syntax alone', async () => {
		await expect(markdown('{{swiftide_sha}}')).rejects.toThrow(/Unknown placeholder \{\{swiftide_sha\}\}/);
		expect(await markdown('```jinja\n{{ question }}\n```')).toBe('```jinja\n{{ question }}\n```');
	});

	it('rewrites docsrs: links and definitions', async () => {
		expect(await markdown('[Agent](docsrs:swiftide/agents/struct.Agent.html)')).toBe(
			'[Agent](https://docs.rs/swiftide/0.33.0/swiftide/agents/struct.Agent.html)',
		);
		expect(await markdown('[core][c]\n\n[c]: docsrs:swiftide_core')).toContain(
			'[c]: https://docs.rs/swiftide-core/0.33.0/swiftide_core/',
		);
		await expect(markdown('[x](docsrs:tokio)')).rejects.toThrow(/Unknown crate "tokio"/);
	});
});

describe('mdx flattening', () => {
	it('drops imports and unwraps known components', async () => {
		const out = await markdown(
			"import { Tabs, TabItem } from '@astrojs/starlight/components';\n\n<Tabs>\n<TabItem label=\"Cargo\">\n\nRun it.\n\n</TabItem>\n</Tabs>\n",
			true,
		);
		expect(out).toBe('**Cargo**\n\nRun it.');
	});

	it('throws on unknown components', async () => {
		await expect(markdown('<Fancy />', true)).rejects.toThrow(/<Fancy> has no markdown equivalent/);
	});
});

describe('versionNote', () => {
	it('describes the frontmatter badges', () => {
		expect(versionNote({ unreleased: true }, '0.33.0')).toBe('_Unreleased: on swiftide master, not yet in 0.33.0._');
		expect(versionNote({ since: '0.32' }, '0.33.0')).toBe('_Available since swiftide 0.32._');
		expect(versionNote(undefined, '0.33.0')).toBeUndefined();
	});
});
