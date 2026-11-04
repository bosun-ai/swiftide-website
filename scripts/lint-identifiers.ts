/**
 * Content lint for swiftide.rs pages.
 *
 * - Every Rust identifier written in inline code must exist in the pinned swiftide source
 *   (see lint/identifiers.ts for what counts as an identifier).
 * - Rust code is never written by hand: ```rust fences, `?raw` imports and `<Code>` are errors;
 *   use `::snippet`.
 * - Every `::snippet` must resolve (file, alias, anchor or lines).
 * - The `types` in src/data/integrations.yaml must exist.
 *
 * Usage: node scripts/lint-identifiers.ts [page.md …]   (paths relative to src/content/docs)
 */
import { readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import type { Root } from 'mdast';
import type { Directives } from 'mdast-util-directive';
import remarkDirective from 'remark-directive';
import remarkFrontmatter from 'remark-frontmatter';
import remarkGfm from 'remark-gfm';
import remarkMdx from 'remark-mdx';
import remarkParse from 'remark-parse';
import { unified } from 'unified';
import { visit } from 'unist-util-visit';
import { parse as parseYaml } from 'yaml';
import { INTEGRATIONS_FILE, loadIntegrations } from '../src/lib/integrations.ts';
import { loadExcerpt, parseSelector, sourceRoots, type SourceRoot } from '../src/lib/snippets.ts';
import { loadSwiftideContext } from '../src/lib/swiftide.ts';
import { parseAllowlist } from './lint/allowlist.ts';
import { buildDefinitionIndex, type DefinitionIndex } from './lint/definition-index.ts';
import { check, classify } from './lint/identifiers.ts';

interface Diagnostic {
	readonly file: string;
	readonly line: number;
	readonly column: number;
	readonly severity: 'error' | 'warning';
	readonly message: string;
}

const projectRoot = path.resolve(import.meta.dirname, '..');
const docsDir = path.join(projectRoot, 'src/content/docs');
const ALLOWLIST = 'scripts/lint-identifiers.allow.txt';

function contentFiles(args: readonly string[]): string[] {
	if (args.length) return args.map((arg) => (path.isAbsolute(arg) ? arg : path.resolve(docsDir, arg)));
	return (readdirSync(docsDir, { recursive: true }) as string[])
		.filter((file) => /\.mdx?$/.test(file))
		.map((file) => path.join(docsDir, file))
		.sort();
}

function parsePage(source: string, mdx: boolean): Root {
	const processor = unified().use(remarkParse).use(remarkFrontmatter).use(remarkGfm).use(remarkDirective);
	return (mdx ? processor.use(remarkMdx) : processor).parse(source) as Root;
}

function pageIgnores(tree: Root): Set<string> {
	const yaml = tree.children.find((node) => node.type === 'yaml');
	const data = yaml && 'value' in yaml ? (parseYaml(yaml.value) as { identLint?: { ignore?: string[] } }) : undefined;
	return new Set(data?.identLint?.ignore ?? []);
}

interface LintContext {
	readonly index: DefinitionIndex;
	readonly allowed: ReadonlySet<string>;
	readonly roots: readonly SourceRoot[];
	/** Allowlist entries that suppressed something; the rest are stale. */
	readonly usedAllowEntries: Set<string>;
}

function checkIdentifier(
	raw: string,
	where: { file: string; line: number; column: number },
	ignored: ReadonlySet<string>,
	ctx: LintContext,
): Diagnostic | undefined {
	const identifier = classify(raw);
	if (!identifier) return undefined;
	const verdict = check(identifier, ctx.index);
	if (verdict.status === 'ok') return undefined;
	if (ignored.has(raw) || ignored.has(identifier.name)) return undefined;
	if (ctx.allowed.has(identifier.name)) {
		ctx.usedAllowEntries.add(identifier.name);
		return undefined;
	}
	if (verdict.status === 'external') {
		return {
			...where,
			severity: 'warning',
			message: `\`${raw}\`: "${verdict.segment}" is used but not defined in swiftide (external type?)`,
		};
	}
	const hint = verdict.suggestion ? ` Did you mean "${verdict.suggestion}"?` : '';
	return { ...where, severity: 'error', message: `\`${raw}\`: "${verdict.segment}" does not exist in swiftide.${hint}` };
}

function lintPage(file: string, ctx: LintContext): Diagnostic[] {
	const source = readFileSync(file, 'utf8');
	const name = path.relative(projectRoot, file);
	const tree = parsePage(source, file.endsWith('.mdx'));
	const ignored = pageIgnores(tree);
	const diagnostics: Diagnostic[] = [];
	const at = (node: { position?: { start: { line: number; column: number } } | undefined }) => ({
		file: name,
		line: node.position?.start.line ?? 1,
		column: node.position?.start.column ?? 1,
	});

	source.split('\n').forEach((line, index) => {
		const match = /\?raw\b|<Code\b/.exec(line);
		if (match) {
			diagnostics.push({
				file: name,
				line: index + 1,
				column: match.index + 1,
				severity: 'error',
				message: `"${match[0]}" imports code by hand; use ::snippet so the code is compiled`,
			});
		}
	});

	visit(tree, (node) => {
		if (node.type === 'inlineCode') {
			const diagnostic = checkIdentifier(node.value, at(node), ignored, ctx);
			if (diagnostic) diagnostics.push(diagnostic);
		} else if (node.type === 'code' && (node.lang === 'rust' || node.lang === 'rs')) {
			diagnostics.push({
				...at(node),
				severity: 'error',
				message: 'Hand-written ```rust block; move the code into snippets/ or an example and use ::snippet',
			});
		} else if (node.type === 'leafDirective' && (node as Directives).name === 'snippet') {
			const attrs = (node as Directives).attributes ?? {};
			try {
				if (!attrs.src) throw new Error('missing src="…"');
				loadExcerpt(attrs.src, parseSelector(attrs.anchor ?? undefined, attrs.lines ?? undefined), ctx.roots);
			} catch (error) {
				diagnostics.push({ ...at(node), severity: 'error', message: `::snippet: ${(error as Error).message}` });
			}
		}
	});
	return diagnostics;
}

function lintIntegrations(ctx: LintContext): Diagnostic[] {
	const lines = readFileSync(path.join(projectRoot, INTEGRATIONS_FILE), 'utf8').split('\n');
	return loadIntegrations(projectRoot).flatMap((integration) =>
		integration.types.flatMap((item) => {
			const line = lines.findIndex((l) => l.includes(item.path)) + 1;
			const diagnostic = checkIdentifier(item.path, { file: INTEGRATIONS_FILE, line, column: 1 }, new Set(), ctx);
			return diagnostic ? [diagnostic] : [];
		}),
	);
}

function report(diagnostics: readonly Diagnostic[]): void {
	const github = process.env.GITHUB_ACTIONS === 'true';
	for (const d of diagnostics) {
		console.log(`${d.file}:${d.line}:${d.column} ${d.severity}: ${d.message}`);
		if (github) console.log(`::${d.severity} file=${d.file},line=${d.line},col=${d.column}::${d.message}`);
	}
}

function main(): void {
	const args = process.argv.slice(2).filter((arg) => arg !== '--');
	const swiftide = loadSwiftideContext(projectRoot);
	const allowlist = parseAllowlist(readFileSync(path.join(projectRoot, ALLOWLIST), 'utf8'));
	const ctx: LintContext = {
		index: buildDefinitionIndex(swiftide.dir, swiftide.crates),
		allowed: new Set(allowlist.entries.map((entry) => entry.identifier)),
		roots: sourceRoots(projectRoot, swiftide),
		usedAllowEntries: new Set(),
	};
	const files = contentFiles(args);
	const diagnostics = [...files.flatMap((file) => lintPage(file, ctx)), ...lintIntegrations(ctx)];
	for (const error of allowlist.errors) {
		diagnostics.push({ file: ALLOWLIST, line: Number(/line (\d+)/.exec(error)?.[1] ?? 1), column: 1, severity: 'error', message: error });
	}
	if (args.length === 0) {
		for (const entry of allowlist.entries) {
			if (!ctx.usedAllowEntries.has(entry.identifier)) {
				diagnostics.push({
					file: ALLOWLIST,
					line: entry.line,
					column: 1,
					severity: 'error',
					message: `stale entry "${entry.identifier}": no page needs it any more; remove it`,
				});
			}
		}
	}
	report(diagnostics);
	const errors = diagnostics.filter((d) => d.severity === 'error').length;
	const warnings = diagnostics.length - errors;
	console.log(`lint:idents: ${files.length} pages, ${errors} errors, ${warnings} warnings`);
	if (errors > 0) process.exit(1);
}

main();
