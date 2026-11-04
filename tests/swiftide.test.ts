import { describe, expect, it } from 'vitest';
import { docsRsUrl, swiftidePermalink } from '../src/lib/swiftide.ts';
import { fixtureSwiftide } from './helpers.ts';

describe('docsRsUrl', () => {
	it('maps crate paths to the pinned version', () => {
		expect(docsRsUrl(fixtureSwiftide, 'swiftide/agents/struct.Agent.html')).toBe(
			'https://docs.rs/swiftide/0.33.0/swiftide/agents/struct.Agent.html',
		);
		expect(docsRsUrl(fixtureSwiftide, 'swiftide_core')).toBe('https://docs.rs/swiftide-core/0.33.0/swiftide_core/');
		expect(docsRsUrl(fixtureSwiftide, 'swiftide_core#traits')).toBe(
			'https://docs.rs/swiftide-core/0.33.0/swiftide_core/#traits',
		);
	});
	it('rejects unknown crates', () => {
		expect(() => docsRsUrl(fixtureSwiftide, 'tokio/index.html')).toThrow(/Unknown crate "tokio"/);
	});
});

describe('swiftidePermalink', () => {
	it('pins the sha', () => {
		expect(swiftidePermalink(fixtureSwiftide, 'examples/a.rs', { start: 1, end: 2 })).toBe(
			`${fixtureSwiftide.ghBlob}examples/a.rs#L1-L2`,
		);
	});
});
