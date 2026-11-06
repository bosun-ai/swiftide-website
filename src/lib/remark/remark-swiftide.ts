/**
 * The swiftide remark plugin: expands every site directive before rendering.
 *
 * One expander, three consumers: the Astro markdown pipeline (`target: 'html'`, Expressive Code
 * renders the meta), the per-page `.md` route via `src/lib/md-renderer.ts` (`target: 'markdown'`)
 * and, through the rendered HTML, `starlight-llms-txt`. Starlight registers `remark-directive`
 * and restores unhandled directives after user plugins, so this plugin only needs to run first.
 *
 * Every failure is fatal and reported at the directive's position.
 */
import type { Root, RootContent } from 'mdast';
import type { Directives } from 'mdast-util-directive';
import { SKIP, visit } from 'unist-util-visit';
import type { VFile } from 'vfile';
import type { Attributes } from './directives/attributes.ts';
import { expandFeatureFlags } from './directives/feature-flags.ts';
import { expandInclude } from './directives/include.ts';
import { expandIntegrations } from './directives/integrations.ts';
import { expandSnippet } from './directives/snippet.ts';
import { expandUnreleased } from './directives/unreleased.ts';
import { createDirectiveEnv, type DirectiveEnv, type RemarkSwiftideOptions } from './env.ts';
import { rewriteInline } from './inline.ts';

export type { RemarkSwiftideOptions } from './env.ts';

type Expander = (attributes: Attributes | null | undefined, env: DirectiveEnv) => RootContent[];

/** Block directives, written on their own line as `::name{…}`. */
const LEAF: Record<string, Expander> = {
	snippet: expandSnippet,
	include: expandInclude,
	'feature-flags': expandFeatureFlags,
	integrations: expandIntegrations,
};

/** Inline directives, written as `:name` inside a paragraph. */
const TEXT: Record<string, Expander> = {
	unreleased: expandUnreleased,
};

function fail(file: VFile, node: Directives, error: unknown): never {
	const reason = error instanceof Error ? error.message : String(error);
	// `file.fail` throws a VFileMessage carrying the file path and the directive's position.
	return file.fail(`swiftide ${node.type === 'textDirective' ? ':' : '::'}${node.name}: ${reason}`, node);
}

function expandDirectives(tree: Root, env: DirectiveEnv, file: VFile): void {
	visit(tree, (node, index, parent) => {
		if (node.type !== 'leafDirective' && node.type !== 'textDirective' && node.type !== 'containerDirective') return;
		if (!parent || index === undefined) return;
		const handlers = node.type === 'leafDirective' ? LEAF : node.type === 'textDirective' ? TEXT : {};
		const expand = handlers[node.name];
		if (!expand) {
			if (node.type === 'leafDirective') {
				fail(file, node, `unknown block directive. Known: ${Object.keys(LEAF).map((n) => `::${n}`).join(', ')}`);
			}
			if (node.type === 'textDirective' && (LEAF[node.name] || node.name in TEXT)) {
				fail(file, node, 'used inline; write it on its own line');
			}
			return;
		}
		let replacement: RootContent[];
		try {
			replacement = expand(node.attributes, env);
		} catch (error) {
			fail(file, node, error);
		}
		(parent.children as RootContent[]).splice(index, 1, ...replacement);
		return [SKIP, index + replacement.length];
	});
}

export default function remarkSwiftide(options: RemarkSwiftideOptions) {
	const env = createDirectiveEnv(options);
	return function transform(tree: Root, file: VFile): void {
		try {
			rewriteInline(tree, env.swiftide);
		} catch (error) {
			file.fail(error instanceof Error ? error.message : String(error));
		}
		expandDirectives(tree, env, file);
	};
}
