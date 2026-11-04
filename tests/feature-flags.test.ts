import { describe, expect, it } from 'vitest';
import { featureNames, loadFeatureFlags } from '../src/lib/feature-flags.ts';
import { fixtureSwiftide } from './helpers.ts';

describe('feature flags', () => {
	const sections = loadFeatureFlags(fixtureSwiftide.dir);

	it('groups flags by #! ### sections', () => {
		expect(sections.map((s) => [s.title, s.flags.map((f) => f.name)])).toEqual([
			[null, ['default', 'macros', 'all']],
			['Integrations', ['qdrant', 'openai', 'anthropic']],
			['Other', ['mcp', 'swiftide-agents']],
		]);
	});

	it('joins multi-line ## docs and ignores plain comments', () => {
		const flags = sections.flatMap((s) => s.flags);
		expect(flags.find((f) => f.name === 'openai')?.doc).toBe('OpenAI with `Responses` support across two lines');
		expect(flags.find((f) => f.name === 'anthropic')?.doc).toBe('');
	});

	it('lists other features a flag enables, across multi-line arrays', () => {
		const flags = sections.flatMap((s) => s.flags);
		expect(flags.find((f) => f.name === 'all')?.enables).toEqual(['qdrant', 'openai']);
		expect(flags.find((f) => f.name === 'mcp')?.enables).toEqual(['swiftide-agents']);
	});

	it('stops at the next table', () => {
		expect(featureNames(sections).size).toBe(8);
	});
});
