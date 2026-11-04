/**
 * Everything the directive expanders need, built once per plugin instance.
 *
 * Data files (feature flags, integrations) are loaded lazily so pages that do not use them do
 * not pay for them, and memoized for the lifetime of the plugin (one build).
 */
import path from 'node:path';
import { featureNames, loadFeatureFlags, type FeatureSection } from '../feature-flags.ts';
import { checkIntegrations, loadIntegrations, type Integration } from '../integrations.ts';
import { sourceRoots, type SourceRoot } from '../snippets.ts';
import type { SwiftideContext } from '../swiftide.ts';
import type { RenderTarget } from './code-meta.ts';

export interface RemarkSwiftideOptions {
	/** Absolute path of the website repository. */
	readonly projectRoot: string;
	readonly swiftide: SwiftideContext;
	readonly target: RenderTarget;
}

export interface DirectiveEnv {
	readonly swiftide: SwiftideContext;
	readonly target: RenderTarget;
	readonly roots: readonly SourceRoot[];
	featureFlags(): readonly FeatureSection[];
	integrations(): readonly Integration[];
}

function memo<T>(load: () => T): () => T {
	let value: { v: T } | undefined;
	return () => (value ??= { v: load() }).v;
}

export function createDirectiveEnv(options: RemarkSwiftideOptions): DirectiveEnv {
	const { projectRoot, swiftide, target } = options;
	const featureFlags = memo(() => loadFeatureFlags(swiftide.dir));
	const integrations = memo(() => {
		const list = loadIntegrations(projectRoot);
		const sections = featureFlags();
		const problems = checkIntegrations(list, {
			features: featureNames(sections),
			requiredFeatures: new Set(
				sections.find((section) => section.title === 'Integrations')?.flags.map((flag) => flag.name) ?? [],
			),
			swiftideDir: swiftide.dir,
			docsDir: path.join(projectRoot, 'src/content/docs'),
		});
		if (problems.length) throw new Error(problems.join('\n'));
		return list;
	});
	return { swiftide, target, roots: sourceRoots(projectRoot, swiftide), featureFlags, integrations };
}
