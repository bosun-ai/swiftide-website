/**
 * Markdown renderer for `starlight-llm-actions` (`renderMarkdown.module`): produces the body of
 * every `/<slug>.md` file from the page source, with all swiftide directives expanded.
 *
 * The plugin prepends `# title` and the description. If this renderer throws, the plugin falls
 * back to the raw source and logs a warning; `scripts/verify-dist.ts` fails the build on any
 * unexpanded directive or JSX that reaches `dist/`.
 */
import type { Root } from 'mdast';
import remarkDirective from 'remark-directive';
import remarkFrontmatter from 'remark-frontmatter';
import remarkGfm from 'remark-gfm';
import remarkMdx from 'remark-mdx';
import remarkParse from 'remark-parse';
import remarkStringify from 'remark-stringify';
import type { MarkdownRenderer } from 'starlight-llm-actions';
import { unified, type Processor } from 'unified';
import { remove } from 'unist-util-remove';
import { VFile } from 'vfile';
import remarkMdxToMarkdown from './remark/mdx-to-markdown.ts';
import remarkSwiftide, { type RemarkSwiftideOptions } from './remark/remark-swiftide.ts';
import { loadSwiftideContext } from './swiftide.ts';

export type MarkdownOptions = Omit<RemarkSwiftideOptions, 'target'>;

function dropFrontmatter() {
	return (tree: Root) => {
		remove(tree, 'yaml');
	};
}

export function createMarkdownProcessor(options: MarkdownOptions, mdx: boolean): Processor<Root, Root, Root, Root, string> {
	const base = unified().use(remarkParse).use(remarkFrontmatter).use(remarkGfm).use(remarkDirective);
	const parsed = mdx ? base.use(remarkMdx) : base;
	const expanded = parsed.use(dropFrontmatter).use(remarkSwiftide, { ...options, target: 'markdown' });
	const flattened = mdx ? expanded.use(remarkMdxToMarkdown, { projectRoot: options.projectRoot }) : expanded;
	return flattened.use(remarkStringify, { fences: true, bullet: '-', emphasis: '_', rule: '-' }) as unknown as Processor<
		Root,
		Root,
		Root,
		Root,
		string
	>;
}

/** Renders one page source to LLM-facing markdown. */
export async function renderPageMarkdown(
	source: string,
	{ path, mdx, options }: { path?: string | undefined; mdx: boolean; options: MarkdownOptions },
): Promise<string> {
	const file = new VFile({ path, value: source });
	const result = await createMarkdownProcessor(options, mdx).process(file);
	return String(result).trimEnd();
}

let siteOptions: MarkdownOptions | undefined;

function options(): MarkdownOptions {
	siteOptions ??= { projectRoot: process.cwd(), swiftide: loadSwiftideContext(process.cwd()) };
	return siteOptions;
}

interface VersionBadges {
	readonly since?: string | undefined;
	readonly unreleased?: boolean | undefined;
}

/** The frontmatter version badges as a line of text, so LLM readers see them too. */
export function versionNote(badges: VersionBadges | undefined, version: string): string | undefined {
	if (badges?.unreleased) return `_Unreleased: on swiftide master, not yet in ${version}._`;
	if (badges?.since) return `_Available since swiftide ${badges.since}._`;
	return undefined;
}

const render: MarkdownRenderer = async (entry) => {
	const filePath = (entry as { filePath?: string }).filePath;
	const body = await renderPageMarkdown(entry.body ?? '', {
		path: filePath,
		mdx: filePath?.endsWith('.mdx') ?? false,
		options: options(),
	});
	const note = versionNote(entry.data['swiftide'] as VersionBadges | undefined, options().swiftide.version);
	return note ? `${note}\n\n${body}` : body;
};

export default render;
