/**
 * Author-facing placeholders that are not directives:
 *
 * - `{{swiftide_version}}` in text, inline code, code blocks, URLs and directive attributes.
 * - `docsrs:<crate>/<path>` link targets, rewritten to the pinned docs.rs version.
 *
 * Any other `{{swiftide_*}}` placeholder is an error, so typos never reach a page.
 */
import type { Root } from 'mdast';
import { visit } from 'unist-util-visit';
import { SnippetError } from '../snippets.ts';
import { docsRsUrl, type SwiftideContext } from '../swiftide.ts';

const PLACEHOLDER = /\{\{\s*(swiftide_[A-Za-z0-9_]*)\s*\}\}/g;

export function substitute(value: string, swiftide: SwiftideContext): string {
	return value.replace(PLACEHOLDER, (_match, name: string) => {
		if (name === 'swiftide_version') return swiftide.version;
		throw new SnippetError(`Unknown placeholder {{${name}}}. Only {{swiftide_version}} exists.`);
	});
}

function rewriteUrl(url: string, swiftide: SwiftideContext): string {
	const substituted = substitute(url, swiftide);
	return substituted.startsWith('docsrs:') ? docsRsUrl(swiftide, substituted.slice('docsrs:'.length)) : substituted;
}

/** Applies placeholder substitution and `docsrs:` rewriting to every node in the tree. */
export function rewriteInline(tree: Root, swiftide: SwiftideContext): void {
	visit(tree, (node) => {
		switch (node.type) {
			case 'text':
			case 'inlineCode':
			case 'code':
			case 'html':
				node.value = substitute(node.value, swiftide);
				break;
			case 'link':
			case 'definition':
			case 'image':
				node.url = rewriteUrl(node.url, swiftide);
				if (node.title) node.title = substitute(node.title, swiftide);
				break;
			case 'textDirective':
			case 'leafDirective':
			case 'containerDirective':
				for (const [key, value] of Object.entries(node.attributes ?? {})) {
					if (typeof value === 'string') node.attributes![key] = substitute(value, swiftide);
				}
				break;
		}
	});
}
