import { describe, expect, it } from 'vitest';
import { docsRsUrl, parsePackageVersion, swiftidePermalink } from '../src/lib/swiftide.ts';
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

describe('parsePackageVersion', () => {
	const workspace = '[workspace]\nmembers = ["swiftide"]\n\n[workspace.package]\nversion = "0.34.0"\nedition = "2024"\n';

	it('reads the version of the package table', () => {
		const crate = '[package]\nname = "swiftide"\nversion = "0.34.1"\n\n[dependencies]\nversion = "9.9.9"\n';
		expect(parsePackageVersion(crate, workspace)).toBe('0.34.1');
	});
	it('follows an inherited workspace version', () => {
		expect(parsePackageVersion('[package]\nname = "swiftide"\nversion.workspace = true\n', workspace)).toBe('0.34.0');
		expect(parsePackageVersion('[package]\nversion = { workspace = true }\n', workspace)).toBe('0.34.0');
	});
	it('returns nothing when no version is declared', () => {
		expect(parsePackageVersion('[package]\nname = "swiftide"\n', workspace)).toBeUndefined();
		expect(parsePackageVersion('[package]\nversion.workspace = true\n', '[workspace]\n')).toBeUndefined();
	});
});
