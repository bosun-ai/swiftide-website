/**
 * The integrations matrix: which feature flag brings which provider or store, what it provides,
 * and where it is shown and explained.
 *
 * `src/data/integrations.yaml` is the single source; it is validated with zod and cross-checked
 * against the pinned swiftide (feature flags, example files) and the site (guide pages).
 */
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { parse } from 'yaml';
import { z } from 'zod';

export const INTEGRATION_KINDS = {
	llm: 'LLM',
	embeddings: 'Embeddings',
	'sparse-embeddings': 'Sparse embeddings',
	structured: 'Structured output',
	storage: 'Storage',
	retrieval: 'Retrieval',
	cache: 'Node cache',
	loader: 'Loader',
	transform: 'Transformers',
	rerank: 'Reranking',
	tooling: 'Agent tooling',
	observability: 'Observability',
} as const;

export type IntegrationKind = keyof typeof INTEGRATION_KINDS;

const kindSchema = z.enum(Object.keys(INTEGRATION_KINDS) as [IntegrationKind, ...IntegrationKind[]]);

const itemSchema = z.strictObject({
	/** Rust path through the `swiftide` facade, e.g. `swiftide::integrations::openai::OpenAI`. */
	path: z.string().regex(/^swiftide(::[A-Za-z_][A-Za-z0-9_]*)+$/),
	kind: z.enum(['struct', 'enum', 'trait', 'type', 'fn', 'mod', 'macro']),
});

const integrationSchema = z.strictObject({
	name: z.string().min(1),
	feature: z.string().regex(/^[a-z0-9-]+$/),
	/** Further feature flags covered by this entry (e.g. `tree-sitter-perl`). */
	alsoFeatures: z.array(z.string()).default([]),
	kinds: z.array(kindSchema).min(1),
	types: z.array(itemSchema).default([]),
	/** Paths relative to the swiftide repository root. */
	examples: z.array(z.string()).default([]),
	/** Docs page slug without leading or trailing slash. */
	page: z.string().regex(/^[a-z0-9-]+(\/[a-z0-9-]+)*$/),
	notes: z.string().optional(),
});

const fileSchema = z.strictObject({ integrations: z.array(integrationSchema).min(1) });

export type Integration = z.infer<typeof integrationSchema>;
export type IntegrationItem = z.infer<typeof itemSchema>;

export const INTEGRATIONS_FILE = 'src/data/integrations.yaml';

export function parseIntegrations(yamlText: string): Integration[] {
	const result = fileSchema.safeParse(parse(yamlText));
	if (!result.success) {
		throw new Error(`${INTEGRATIONS_FILE} is invalid:\n${z.prettifyError(result.error)}`);
	}
	return result.data.integrations;
}

export interface IntegrationChecks {
	/** Every feature flag of the swiftide crate. */
	readonly features: ReadonlySet<string>;
	/** Flags that must be covered by an entry (the "Integrations" section of Cargo.toml). */
	readonly requiredFeatures: ReadonlySet<string>;
	readonly swiftideDir: string;
	readonly docsDir: string;
}

/** Cross-checks entries against swiftide and the site. Returns problems; empty means valid. */
export function checkIntegrations(integrations: readonly Integration[], checks: IntegrationChecks): string[] {
	const problems: string[] = [];
	const covered = new Set<string>();
	for (const integration of integrations) {
		const where = `${INTEGRATIONS_FILE} (${integration.name})`;
		for (const feature of [integration.feature, ...integration.alsoFeatures]) {
			if (!checks.features.has(feature)) problems.push(`${where}: unknown feature flag "${feature}"`);
			if (covered.has(feature)) problems.push(`${where}: feature "${feature}" is listed twice`);
			covered.add(feature);
		}
		for (const example of integration.examples) {
			if (!existsSync(path.join(checks.swiftideDir, example))) {
				problems.push(`${where}: example "${example}" does not exist in vendor/swiftide`);
			}
		}
		const page = path.join(checks.docsDir, integration.page);
		if (!existsSync(`${page}.md`) && !existsSync(`${page}.mdx`)) {
			problems.push(`${where}: page "${integration.page}" does not exist`);
		}
	}
	for (const feature of checks.requiredFeatures) {
		if (!covered.has(feature)) problems.push(`${INTEGRATIONS_FILE}: feature "${feature}" has no entry`);
	}
	return problems;
}

export function loadIntegrations(projectRoot: string): Integration[] {
	return parseIntegrations(readFileSync(path.join(projectRoot, INTEGRATIONS_FILE), 'utf8'));
}

/** The integrations behind the given feature flags, in that order. Unknown flags throw. */
export function integrationsByFeature(integrations: readonly Integration[], features: readonly string[]): Integration[] {
	return features.map((feature) => {
		const found = integrations.find((integration) => integration.feature === feature);
		if (!found) throw new Error(`"${feature}" is not a feature in ${INTEGRATIONS_FILE}`);
		return found;
	});
}

/** docs.rs target (for `docsRsUrl`) of an item path such as `swiftide::agents::Agent`. */
export function docsRsTarget(item: IntegrationItem): string {
	const segments = item.path.split('::');
	const name = segments.pop()!;
	if (item.kind === 'mod') return `${[...segments, name].join('/')}/index.html`;
	return `${segments.join('/')}/${item.kind}.${name}.html`;
}
