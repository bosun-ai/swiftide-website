import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { SwiftideContext } from '../src/lib/swiftide.ts';

export const FIXTURE_ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), 'fixtures', 'project');

/** A swiftide context for the fixture project, without touching git. */
export const fixtureSwiftide: SwiftideContext = {
	dir: path.join(FIXTURE_ROOT, 'vendor', 'swiftide'),
	sha: '0123456789abcdef0123456789abcdef01234567',
	shortSha: '012345678',
	tag: 'v0.33.0',
	version: '0.33',
	crates: ['swiftide', 'swiftide_agents', 'swiftide_core'],
	docsRsBase: 'https://docs.rs/swiftide/0.33/swiftide/',
	ghBlob: 'https://github.com/bosun-ai/swiftide/blob/0123456789abcdef0123456789abcdef01234567/',
};
