/**
 * Dev-only integration: when an excerpted source or a generator input changes, re-sync the
 * content layer (page digests include those files, see `content-loader.ts`) and reload.
 */
import type { AstroIntegration } from 'astro';
import { watchedPaths } from './page-dependencies.ts';
import { loadSwiftideContext } from './swiftide.ts';

export function snippetWatch(projectRoot: string = process.cwd()): AstroIntegration {
	return {
		name: 'swiftide-snippet-watch',
		hooks: {
			'astro:server:setup': ({ server, refreshContent, logger }) => {
				const paths = watchedPaths(projectRoot, loadSwiftideContext(projectRoot));
				server.watcher.add(paths);
				const relevant = (file: string) => paths.some((p) => file === p || file.startsWith(`${p}/`));
				let pending: ReturnType<typeof setTimeout> | undefined;
				const onChange = (file: string) => {
					if (!relevant(file)) return;
					clearTimeout(pending);
					pending = setTimeout(async () => {
						logger.info(`${file} changed, refreshing docs`);
						await refreshContent?.({});
						// MDX pages render lazily as Vite modules; drop them so they re-expand too.
						for (const mod of server.moduleGraph.idToModuleMap.values()) {
							if (mod.id?.endsWith('.mdx')) server.moduleGraph.invalidateModule(mod);
						}
						server.ws.send({ type: 'full-reload' });
					}, 100);
				};
				server.watcher.on('change', onChange);
				server.watcher.on('add', onChange);
				server.watcher.on('unlink', onChange);
			},
		},
	};
}
