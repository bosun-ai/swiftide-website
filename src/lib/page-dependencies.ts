/**
 * Which files outside `src/content` a page's rendering depends on.
 *
 * Astro caches rendered content by a digest of the page source, so a change to an excerpted
 * Rust file would otherwise never reach the page. The docs loader mixes these files' contents
 * into each page's digest, and the dev watcher uses the same roots to trigger a refresh.
 */
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { INTEGRATIONS_FILE } from './integrations.ts';
import { resolveSource, type SourceRoot } from './snippets.ts';
import type { SwiftideContext } from './swiftide.ts';

const SRC_ATTRIBUTE = /::(?:snippet|include)\{[^}\n]*\bsrc="(@[^"]+)"/g;

/** Absolute paths of every file the page source pulls in. Unresolvable refs are skipped here; the remark plugin reports them. */
export function pageDependencies(
	source: string,
	{ projectRoot, swiftide, roots }: { projectRoot: string; swiftide: SwiftideContext; roots: readonly SourceRoot[] },
): string[] {
	const files = new Set<string>();
	for (const [, src] of source.matchAll(SRC_ATTRIBUTE)) {
		try {
			files.add(resolveSource(src!, roots).absPath);
		} catch {
			// Reported with a position by the remark plugin.
		}
	}
	if (source.includes('::feature-flags') || source.includes('::integrations')) {
		files.add(path.join(swiftide.dir, 'swiftide', 'Cargo.toml'));
	}
	if (source.includes('::integrations')) files.add(path.join(projectRoot, INTEGRATIONS_FILE));
	return [...files].sort();
}

/** A string that changes whenever the pinned commit or any dependency of the page changes. */
export function dependencyFingerprint(files: readonly string[], swiftide: SwiftideContext): string {
	const hash = createHash('sha1').update(swiftide.sha);
	for (const file of files) {
		hash.update(file);
		if (existsSync(file)) hash.update(readFileSync(file));
	}
	return hash.digest('hex');
}

/** Directories and files whose changes can affect rendered pages (watched in dev). */
export function watchedPaths(projectRoot: string, swiftide: SwiftideContext): string[] {
	return [
		path.join(swiftide.dir, 'examples'),
		path.join(swiftide.dir, 'swiftide', 'Cargo.toml'),
		path.join(swiftide.dir, 'CHANGELOG.md'),
		path.join(projectRoot, 'snippets', 'src'),
		path.join(projectRoot, INTEGRATIONS_FILE),
	];
}
