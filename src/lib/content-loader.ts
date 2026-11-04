/**
 * Starlight's docs loader, with each page's digest extended by the files it excerpts.
 *
 * Without this, Astro's content cache keeps serving a page whose `::snippet` source changed.
 */
import { docsLoader } from '@astrojs/starlight/loaders';
import type { Loader, LoaderContext } from 'astro/loaders';
import { dependencyFingerprint, pageDependencies } from './page-dependencies.ts';
import { sourceRoots } from './snippets.ts';
import { loadSwiftideContext } from './swiftide.ts';

export function swiftideDocsLoader(projectRoot: string = process.cwd()): Loader {
	const inner = docsLoader();
	return {
		name: 'swiftide-docs-loader',
		load: (context: LoaderContext) => {
			const swiftide = loadSwiftideContext(projectRoot);
			const deps = { projectRoot, swiftide, roots: sourceRoots(projectRoot, swiftide) };
			return inner.load({
				...context,
				generateDigest: (data) => {
					if (typeof data !== 'string') return context.generateDigest(data);
					const files = pageDependencies(data, deps);
					return context.generateDigest(`${data}\0${dependencyFingerprint(files, swiftide)}`);
				},
			});
		},
	};
}
