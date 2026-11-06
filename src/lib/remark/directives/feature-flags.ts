/**
 * `::feature-flags{level}`
 *
 * One heading and table per section of the swiftide crate's `[features]` table.
 */
import type { Heading, RootContent } from 'mdast';
import { SnippetError } from '../../snippets.ts';
import type { DirectiveEnv } from '../env.ts';
import { code, joined, parseInline, table, text } from '../mdast.ts';
import { checkAttributes, type Attributes } from './attributes.ts';

export function expandFeatureFlags(attributes: Attributes | null | undefined, env: DirectiveEnv): RootContent[] {
	const attrs = checkAttributes('feature-flags', attributes, ['level']);
	const depth = Number(attrs.level ?? 2);
	if (![2, 3, 4].includes(depth)) throw new SnippetError(`level="${attrs.level}" must be 2, 3 or 4`);
	return env.featureFlags().flatMap((section): RootContent[] => {
		const heading: Heading = {
			type: 'heading',
			depth: depth as Heading['depth'],
			children: [text(section.title ?? 'General')],
		};
		const rows = section.flags.map((flag) => [
			[code(flag.name)],
			flag.doc ? parseInline(flag.doc) : [text('Undocumented')],
			joined(flag.enables.map((name) => [code(name)])),
		]);
		return [heading, table(['Flag', 'Description', 'Also enables'], rows)];
	});
}
