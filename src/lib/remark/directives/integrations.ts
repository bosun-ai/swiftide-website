/**
 * `::integrations{kind}`
 *
 * Without `kind`: the full matrix (every integration and what it provides). With `kind`: only
 * the integrations that provide it.
 */
import type { RootContent } from 'mdast';
import { docsRsTarget, INTEGRATION_KINDS, type Integration, type IntegrationKind } from '../../integrations.ts';
import { SnippetError } from '../../snippets.ts';
import { docsRsUrl, swiftidePermalink } from '../../swiftide.ts';
import type { DirectiveEnv } from '../env.ts';
import { code, joined, link, parseInline, table, text, type Cell } from '../mdast.ts';
import { checkAttributes, type Attributes } from './attributes.ts';

function isKind(kind: string): kind is IntegrationKind {
	return Object.hasOwn(INTEGRATION_KINDS, kind);
}

function cells(integration: Integration, env: DirectiveEnv) {
	const types = joined(
		integration.types.map((item) => [
			link(docsRsUrl(env.swiftide, docsRsTarget(item)), [code(item.path.split('::').at(-1)!)]),
		]),
	);
	const examples = joined(
		integration.examples.map((example) => [
			link(swiftidePermalink(env.swiftide, example), [code(example.replace(/^examples\//, ''))]),
		]),
	);
	return {
		name: integration.notes ? [text(integration.name), text(': '), ...parseInline(integration.notes)] : [text(integration.name)],
		flag: [code(integration.feature)] as Cell,
		provides: [text(integration.kinds.map((kind) => INTEGRATION_KINDS[kind]).join(', '))] as Cell,
		types: types.length ? types : [text('-')],
		examples: examples.length ? examples : [text('-')],
		guide: [link(`/${integration.page}/`, [text(integration.page)])] as Cell,
	};
}

export function expandIntegrations(attributes: Attributes | null | undefined, env: DirectiveEnv): RootContent[] {
	const attrs = checkAttributes('integrations', attributes, ['kind']);
	const kind = attrs.kind;
	if (kind !== undefined && !isKind(kind)) {
		throw new SnippetError(`kind="${kind}" must be one of ${Object.keys(INTEGRATION_KINDS).join(', ')}`);
	}
	const selected = env.integrations().filter((i) => kind === undefined || i.kinds.includes(kind));
	if (selected.length === 0) throw new SnippetError(`No integration provides kind="${kind}"`);
	const rows = selected.map((integration) => cells(integration, env));
	if (kind === undefined) {
		return [
			table(
				['Integration', 'Feature flag', 'Provides', 'Main types', 'Examples', 'Guide'],
				rows.map((r) => [r.name, r.flag, r.provides, r.types, r.examples, r.guide]),
			),
		];
	}
	return [
		table(
			['Integration', 'Feature flag', 'Main types', 'Examples', 'Guide'],
			rows.map((r) => [r.name, r.flag, r.types, r.examples, r.guide]),
		),
	];
}
