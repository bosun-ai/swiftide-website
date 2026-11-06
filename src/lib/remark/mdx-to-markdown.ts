/**
 * Flattens MDX to plain markdown for the `.md` route: drops ESM, unwraps the Starlight
 * components we allow in content and throws on anything else, so an unknown component never
 * leaks into what LLMs read.
 */
import type { BlockContent, Heading, Paragraph, PhrasingContent, Root, RootContent } from 'mdast';
import type { MdxJsxAttribute, MdxJsxFlowElement, MdxJsxTextElement } from 'mdast-util-mdx-jsx';
import { SKIP, visit } from 'unist-util-visit';
import type { VFile } from 'vfile';
import { loadReleaseRun } from '../release-run.ts';

type JsxElement = MdxJsxFlowElement | MdxJsxTextElement;

function attr(node: JsxElement, name: string): string | undefined {
	const found = node.attributes.find((a): a is MdxJsxAttribute => a.type === 'mdxJsxAttribute' && a.name === name);
	return typeof found?.value === 'string' ? found.value : undefined;
}

const strong = (value: string): PhrasingContent => ({ type: 'strong', children: [{ type: 'text', value }] });
const paragraph = (children: PhrasingContent[]): Paragraph => ({ type: 'paragraph', children });
const heading = (depth: Heading['depth'], value: string): Heading => ({ type: 'heading', depth, children: [{ type: 'text', value }] });
const link = (url: string, value: string): PhrasingContent => ({ type: 'link', url, children: [{ type: 'text', value }] });
const text = (value: string): PhrasingContent => ({ type: 'text', value });

export interface MdxToMarkdownOptions {
	/** Site root, for components whose content is read from files (the home page's recorded run). */
	readonly projectRoot: string;
}

type Rule = (node: JsxElement, options: MdxToMarkdownOptions) => RootContent[];

/** The home page's components (src/components/home), as the headings, paragraphs and lists they show. */
const HOME_COMPONENTS: Record<string, Rule> = {
	HeroStage: (node, { projectRoot }) => [
		paragraph([link(attr(node, 'href') ?? '', attr(node, 'action') ?? '')]),
		...(node.children as RootContent[]),
		{ type: 'code', lang: 'text', value: loadReleaseRun(projectRoot).join('\n') },
	],
	Showcase: (node) => node.children as RootContent[],
	ShowcasePanel: (node) => {
		const example = attr(node, 'example') ?? '';
		return [
			heading(2, `${attr(node, 'label') ?? ''}: ${attr(node, 'title') ?? ''}`),
			paragraph([text(attr(node, 'description') ?? '')]),
			...(node.children as RootContent[]),
			{ type: 'code', lang: 'text', value: attr(node, 'result') ?? '' },
			paragraph([link(attr(node, 'guide') ?? '', 'Read the guide'), text(' · '), link(`/showcases/${example}/`, 'Run it')]),
		];
	},
	Composition: (node) => node.children as RootContent[],
	IntegrationTiles: (node) => {
		const features = (attr(node, 'features') ?? '').split(/\s+/).filter(Boolean);
		return [
			heading(2, attr(node, 'heading') ?? ''),
			paragraph(features.flatMap((feature, i) => [...(i ? [text(' ')] : []), { type: 'inlineCode' as const, value: feature }])),
			paragraph([link(attr(node, 'href') ?? '', 'All integrations')]),
		];
	},
	StartHere: (node) => node.children as RootContent[],
};

/** What each allowed component becomes. Children are kept unless the rule replaces them. */
const COMPONENTS: Record<string, Rule> = {
	...HOME_COMPONENTS,
	Tabs: (node) => node.children as RootContent[],
	TabItem: (node) => [paragraph([strong(attr(node, 'label') ?? 'Tab')]), ...(node.children as RootContent[])],
	Steps: (node) => node.children as RootContent[],
	CardGrid: (node) => node.children as RootContent[],
	FileTree: (node) => node.children as RootContent[],
	Card: (node) => [paragraph([strong(attr(node, 'title') ?? '')]), ...(node.children as RootContent[])],
	LinkCard: (node) => {
		const description = attr(node, 'description');
		return [
			paragraph([
				{ type: 'link', url: attr(node, 'href') ?? '', children: [{ type: 'text', value: attr(node, 'title') ?? '' }] },
				...(description ? [{ type: 'text' as const, value: `: ${description}` }] : []),
			]),
		];
	},
	Aside: (node) => [
		{
			type: 'blockquote',
			children: [
				paragraph([strong(`${attr(node, 'title') ?? attr(node, 'type') ?? 'note'}:`)]),
				...(node.children as BlockContent[]),
			],
		},
	],
	Badge: (node) => [{ type: 'text', value: attr(node, 'text') ?? '' }],
};

const isComment = (value: string) => /^\s*\/\*[\s\S]*\*\/\s*$/.test(value);

export default function remarkMdxToMarkdown(options: MdxToMarkdownOptions) {
	return function transform(tree: Root, file: VFile): void {
		visit(tree, (node, index, parent) => {
			if (!parent || index === undefined) return;
			const children = parent.children as RootContent[];
			switch (node.type) {
				case 'mdxjsEsm':
					children.splice(index, 1);
					return [SKIP, index];
				case 'mdxFlowExpression':
				case 'mdxTextExpression':
					if (!isComment(node.value)) file.fail(`MDX expression {${node.value}} has no markdown equivalent`, node);
					children.splice(index, 1);
					return [SKIP, index];
				case 'mdxJsxFlowElement':
				case 'mdxJsxTextElement': {
					const rule = node.name ? COMPONENTS[node.name] : undefined;
					if (!rule) {
						return file.fail(
							`<${node.name ?? 'fragment'}> has no markdown equivalent. Allowed: ${Object.keys(COMPONENTS).join(', ')}`,
							node,
						);
					}
					children.splice(index, 1, ...rule(node, options));
					return index;
				}
				default:
					return;
			}
		});
	};
}
