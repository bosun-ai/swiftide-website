import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { checkIntegrations, docsRsTarget, loadIntegrations, parseIntegrations } from '../src/lib/integrations.ts';
import { FIXTURE_ROOT, fixtureSwiftide } from './helpers.ts';

const checks = {
	features: new Set(['openai', 'qdrant', 'anthropic', 'mcp']),
	requiredFeatures: new Set(['openai', 'qdrant', 'anthropic']),
	swiftideDir: fixtureSwiftide.dir,
	docsDir: path.join(FIXTURE_ROOT, 'src/content/docs'),
};

describe('integrations', () => {
	it('loads and passes the cross-checks', () => {
		const list = loadIntegrations(FIXTURE_ROOT);
		expect(list.map((i) => i.feature)).toEqual(['openai', 'qdrant', 'anthropic']);
		expect(checkIntegrations(list, checks)).toEqual([]);
	});

	it('reports unknown flags, missing examples, pages and uncovered flags', () => {
		const list = parseIntegrations(`integrations:
  - { name: X, feature: nope, kinds: [llm], examples: [examples/missing.rs], page: integrations/missing }
`);
		expect(checkIntegrations(list, checks)).toEqual([
			'src/data/integrations.yaml (X): unknown feature flag "nope"',
			'src/data/integrations.yaml (X): example "examples/missing.rs" does not exist in vendor/swiftide',
			'src/data/integrations.yaml (X): page "integrations/missing" does not exist',
			'src/data/integrations.yaml: feature "openai" has no entry',
			'src/data/integrations.yaml: feature "qdrant" has no entry',
			'src/data/integrations.yaml: feature "anthropic" has no entry',
		]);
	});

	it('rejects unknown kinds and keys', () => {
		expect(() => parseIntegrations('integrations:\n  - { name: X, feature: x, kinds: [magic], page: a }')).toThrow(/kinds/);
		expect(() => parseIntegrations('integrations:\n  - { name: X, feature: x, kinds: [llm], page: a, extra: 1 }')).toThrow(
			/extra/,
		);
	});

	it('builds docs.rs targets', () => {
		expect(docsRsTarget({ path: 'swiftide::integrations::openai::OpenAI', kind: 'type' })).toBe(
			'swiftide/integrations/openai/type.OpenAI.html',
		);
		expect(docsRsTarget({ path: 'swiftide::agents', kind: 'mod' })).toBe('swiftide/agents/index.html');
	});
});
