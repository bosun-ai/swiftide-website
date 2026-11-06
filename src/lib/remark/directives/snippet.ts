/**
 * `::snippet{src anchor|lines title collapse mark ins del frame wrap source lang}`
 *
 * Expands to a code block cut from a compiled source file, followed by a "Source:" permalink to
 * the exact lines at the pinned commit (unless `source="false"`).
 */
import type { Code, Paragraph, RootContent } from 'mdast';
import { languageFor, loadExcerpt, parseSelector, type Excerpt } from '../../snippets.ts';
import { codeMeta, parseCodeDisplay } from '../code-meta.ts';
import type { DirectiveEnv } from '../env.ts';
import { code, link, text } from '../mdast.ts';
import { checkAttributes, type Attributes } from './attributes.ts';

const ATTRIBUTES = ['src', 'anchor', 'lines', 'title', 'collapse', 'mark', 'ins', 'del', 'frame', 'wrap', 'source', 'lang'];

function sourceLine(excerpt: Excerpt, wholeFile: boolean): Paragraph {
	const { root, relPath } = excerpt.source;
	const lines = wholeFile ? undefined : { start: excerpt.startLine, end: excerpt.endLine };
	const label = root.label(relPath) + (lines ? `#L${lines.start}-L${lines.end}` : '');
	return {
		type: 'paragraph',
		data: { hProperties: { className: ['sw-source'] } },
		children: [text('Source: '), link(root.permalink(relPath, lines), [code(label)])],
	};
}

export function expandSnippet(attributes: Attributes | null | undefined, env: DirectiveEnv): RootContent[] {
	const attrs = checkAttributes('snippet', attributes, ATTRIBUTES, ['src']);
	const selector = parseSelector(attrs.anchor, attrs.lines);
	const excerpt = loadExcerpt(attrs.src!, selector, env.roots);
	const display = parseCodeDisplay(attrs, excerpt.lineCount);
	const block: Code = {
		type: 'code',
		lang: attrs.lang || languageFor(excerpt.source.relPath),
		meta: codeMeta(display, env.target) ?? null,
		value: excerpt.code,
	};
	if (attrs.source === 'false') return [block];
	return [block, sourceLine(excerpt, selector.kind === 'file')];
}
