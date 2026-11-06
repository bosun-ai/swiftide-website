/**
 * `::include{src shift strip llms}`
 *
 * Splices a markdown file from an allowed root into the page. `strip` is a regular expression;
 * matching lines are dropped before parsing. `shift` demotes every heading by that many levels.
 * `llms="false"` keeps the included content out of `/llms-*.txt` (it stays on the page and in
 * the page's `.md`), for large generated content such as the changelog.
 */
import { readFileSync } from 'node:fs';
import type { BlockContent, RootContent } from 'mdast';
import { visit } from 'unist-util-visit';
import { resolveSource, SnippetError } from '../../snippets.ts';
import type { DirectiveEnv } from '../env.ts';
import { parseMarkdown } from '../mdast.ts';
import { checkAttributes, type Attributes } from './attributes.ts';

/** Elements with this class are removed by starlight-llms-txt (`customSelectors` in astro.config.mjs). */
export const LLMS_EXCLUDE_CLASS = 'sw-llms-exclude';

export function expandInclude(attributes: Attributes | null | undefined, env: DirectiveEnv): RootContent[] {
	const attrs = checkAttributes('include', attributes, ['src', 'shift', 'strip', 'llms'], ['src']);
	const shift = attrs.shift === undefined ? 0 : Number(attrs.shift);
	if (!Number.isInteger(shift) || shift < 0 || shift > 5) {
		throw new SnippetError(`shift="${attrs.shift}" must be an integer from 0 to 5`);
	}
	let strip: RegExp | undefined;
	try {
		strip = attrs.strip === undefined ? undefined : new RegExp(attrs.strip);
	} catch (error) {
		throw new SnippetError(`strip="${attrs.strip}" is not a valid regular expression: ${String(error)}`);
	}
	const source = resolveSource(attrs.src!, env.roots);
	const lines = readFileSync(source.absPath, 'utf8').split('\n');
	const tree = parseMarkdown(lines.filter((line) => !strip?.test(line)).join('\n'));
	if (tree.children.length === 0) throw new SnippetError(`${attrs.src} is empty after strip`);
	if (shift > 0) {
		visit(tree, 'heading', (heading) => {
			heading.depth = Math.min(6, heading.depth + shift) as typeof heading.depth;
		});
	}
	if (attrs.llms !== undefined && attrs.llms !== 'false') throw new SnippetError(`llms="${attrs.llms}" must be "false"`);
	if (attrs.llms === undefined || env.target === 'markdown') return tree.children;
	return [
		{
			type: 'blockquote',
			data: { hName: 'div', hProperties: { className: [LLMS_EXCLUDE_CLASS] } },
			children: tree.children as BlockContent[],
		},
	];
}
