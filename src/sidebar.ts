/**
 * The site navigation (plan §2). Groups map to content directories; pages are ordered by their
 * `sidebar.order` frontmatter. Writers add pages only from the manifest.
 */
import type { StarlightUserConfig } from '@astrojs/starlight/types';
import type { SwiftideContext } from './lib/swiftide.ts';

type Sidebar = NonNullable<StarlightUserConfig['sidebar']>;

const section = (label: string, directory: string, collapsed = true) => ({
	label,
	collapsed,
	items: [{ autogenerate: { directory } }],
});

export function sidebar(swiftide: SwiftideContext): Sidebar {
	return [
		section('Start', 'start', false),
		section('Agents', 'agents'),
		section('Tasks', 'tasks'),
		section('Indexing', 'indexing'),
		section('Querying', 'querying'),
		section('Models and prompts', 'models'),
		section('Observability', 'observability'),
		section('Integrations', 'integrations'),
		section('Showcases', 'showcases'),
		section('Cookbook', 'cookbook'),
		{
			label: 'Reference',
			collapsed: true,
			items: [
				{ autogenerate: { directory: 'reference' } },
				{
					label: 'API (docs.rs)',
					link: swiftide.docsRsBase,
					badge: { text: swiftide.version, variant: 'note' },
					attrs: { target: '_blank', rel: 'noopener' },
				},
				{ label: 'llms.txt', link: '/llms.txt' },
			],
		},
		section('Community', 'community'),
	];
}
