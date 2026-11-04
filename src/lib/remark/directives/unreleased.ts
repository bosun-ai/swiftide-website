/**
 * `:unreleased` (inline)
 *
 * Marks an API that exists on swiftide master but not in the latest release.
 */
import type { PhrasingContent } from 'mdast';
import type { DirectiveEnv } from '../env.ts';
import { text } from '../mdast.ts';
import { checkAttributes, type Attributes } from './attributes.ts';

export function expandUnreleased(attributes: Attributes | null | undefined, env: DirectiveEnv): PhrasingContent[] {
	checkAttributes('unreleased', attributes, []);
	const explanation = `On swiftide master, not yet in ${env.swiftide.version}`;
	if (env.target === 'markdown') return [text(`(unreleased: ${explanation.toLowerCase()})`)];
	return [
		{
			type: 'emphasis',
			data: {
				hName: 'span',
				hProperties: { className: ['sw-badge', 'sw-badge--unreleased'], title: explanation },
			},
			children: [text('unreleased')],
		},
	];
}
