import { describe, expect, it } from 'vitest';
import { parseAllowlist } from '../scripts/lint/allowlist.ts';
import type { DefinitionIndex } from '../scripts/lint/definition-index.ts';
import { check, classify, suggest } from '../scripts/lint/identifiers.ts';

const index: DefinitionIndex = {
	defined: new Set(['swiftide', 'agents', 'Agent', 'AgentBuilder', 'builder', 'ChatCompletion', 'then_chunk']),
	words: new Set(['HashMap']),
	externalRoots: new Set(['std', 'tokio']),
};

describe('classify', () => {
	it.each([
		['swiftide::agents::Agent', 'path'],
		['Agent::builder()', 'path'],
		['ChatCompletion', 'type'],
		['Node<String>', undefined],
		['HashMap<String, Value>', 'type'],
		['then_chunk', 'method'],
		['.run()', 'method'],
		['tokio::spawn', 'path'],
		['cargo add swiftide', undefined],
		['OPENAI_API_KEY', undefined],
		['Agent', undefined],
		['swiftide-agents', undefined],
		['Cargo.toml', undefined],
	])('%s -> %s', (raw, kind) => {
		expect(classify(raw)?.kind).toBe(kind);
	});
});

describe('check', () => {
	const verdict = (raw: string) => check(classify(raw)!, index);
	it('accepts defined identifiers and external roots', () => {
		expect(verdict('swiftide::agents::Agent')).toEqual({ status: 'ok' });
		expect(verdict('Agent::builder()')).toEqual({ status: 'ok' });
		expect(verdict('tokio::spawn')).toEqual({ status: 'ok' });
	});
	it('downgrades names that are only used in swiftide', () => {
		expect(verdict('HashMap')).toEqual({ status: 'external', segment: 'HashMap' });
	});
	it('suggests close matches for unknown names', () => {
		expect(verdict('ChatCompletoin')).toEqual({ status: 'unknown', segment: 'ChatCompletoin', suggestion: 'ChatCompletion' });
		expect(verdict('swiftide::agents::Agnet')).toMatchObject({ segment: 'Agnet', suggestion: 'Agent' });
		expect(suggest('CompletelyDifferent', index.defined)).toBeUndefined();
	});
});

describe('parseAllowlist', () => {
	it('requires a reason per entry', () => {
		const { entries, errors } = parseAllowlist('# comment\nFoo # re-exported from bar\n\nBar\n');
		expect(entries).toEqual([{ identifier: 'Foo', reason: 're-exported from bar', line: 2 }]);
		expect(errors).toEqual(['line 4: "Bar" needs a "# reason"']);
	});
});
