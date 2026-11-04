/**
 * Checks the built site in `dist/` for the guarantees the build itself cannot enforce:
 *
 * - every page has a `.md` sibling and advertises it with `<link rel="alternate">`;
 * - no unexpanded directive or placeholder reached any output file;
 * - the `.md` of every page with `::snippet` contains the expanded code;
 * - llms.txt, llms-full.txt and llms-small.txt exist, contain real code, and small stays small;
 * - every URL of the old site still resolves, directly or through a redirect.
 *
 * Usage: node scripts/verify-dist.ts [distDir]
 */
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';

const SITE = 'https://swiftide.rs';
const LLMS_SMALL_LIMIT = 400 * 1024;
const projectRoot = path.resolve(import.meta.dirname, '..');
const dist = path.resolve(process.argv[2] ?? path.join(projectRoot, 'dist'));
const docsDir = path.join(projectRoot, 'src/content/docs');

type Problems = string[];

function filesUnder(dir: string, extensions: readonly string[]): string[] {
	return (readdirSync(dir, { recursive: true }) as string[])
		.filter((file) => extensions.some((ext) => file.endsWith(ext)))
		.map((file) => path.join(dir, file));
}

const rel = (file: string) => path.relative(dist, file);
const read = (file: string) => readFileSync(file, 'utf8');
const REDIRECT = /<meta http-equiv="refresh" content="\d+;\s*url=([^"]+)"/i;
const isRedirect = (html: string) => REDIRECT.test(html);

/** Path of the HTML file serving a URL path like `/start/installation/`. */
function htmlFor(urlPath: string): string {
	return path.join(dist, urlPath.replace(/^\/|\/$/g, ''), 'index.html');
}

/** `/start/installation/index.html` -> `/start/installation.md`; `/index.html` -> `/index.md`. */
function markdownUrlFor(htmlFile: string): string {
	const dir = path.dirname(rel(htmlFile)).split(path.sep).join('/');
	return dir === '.' ? '/index.md' : `/${dir}.md`;
}

function checkPages(htmlFiles: readonly string[]): Problems {
	const problems: Problems = [];
	let pages = 0;
	for (const file of htmlFiles) {
		const name = rel(file);
		if (name === '404.html' || name.startsWith('pagefind') || name.startsWith('_astro')) continue;
		const html = read(file);
		if (isRedirect(html)) continue;
		pages++;
		const mdUrl = markdownUrlFor(file);
		const alternate = `<link rel="alternate" type="text/markdown" href="${SITE}${mdUrl}"`;
		if (!html.includes(alternate)) problems.push(`${name}: missing ${alternate}>`);
		if (!existsSync(path.join(dist, mdUrl))) problems.push(`${name}: missing markdown sibling ${mdUrl}`);
	}
	if (pages === 0) problems.push('no pages found in dist');
	return problems;
}

/** Directive and placeholder syntax that must never survive the build. */
const LEFTOVERS: readonly [RegExp, string][] = [
	// The lookbehind skips Rust paths such as `swiftide::integrations::openai`.
	[/(?<![\w:])::(?:snippet|include|feature-flags|integrations)\b/, 'unexpanded block directive'],
	[/(?:^|[\s>(]):unreleased\b/m, 'unexpanded :unreleased'],
	[/\{\{\s*swiftide_[a-z_]*\s*\}\}/, 'unexpanded {{swiftide_*}} placeholder'],
	[/\bdocsrs:[a-z]/, 'unrewritten docsrs: link'],
];

function lineOf(text: string, index: number): number {
	return text.slice(0, index).split('\n').length;
}

function checkLeftovers(files: readonly string[]): Problems {
	const problems: Problems = [];
	for (const file of files) {
		const text = read(file);
		for (const [pattern, label] of LEFTOVERS) {
			const match = pattern.exec(text);
			if (match) problems.push(`${rel(file)}:${lineOf(text, match.index)}: ${label}: ${match[0].trim()}`);
		}
	}
	return problems;
}

/** Text outside fenced code blocks, so code samples cannot trip the prose checks. */
function prose(markdown: string): string {
	return markdown.replace(/^(```+|~~~+)[^\n]*\n[\s\S]*?^\1[ \t]*$/gm, '');
}

function fenceCount(markdown: string): number {
	return (markdown.match(/^```+[a-z]/gm) ?? []).length;
}

/** The `.md` route falls back to raw source when rendering fails; catch MDX that leaked through. */
function checkMarkdownSiblings(): Problems {
	const problems: Problems = [];
	for (const source of filesUnder(docsDir, ['.md', '.mdx'])) {
		const id = path.relative(docsDir, source).replace(/\.mdx?$/, '').split(path.sep).join('/');
		const mdFile = path.join(dist, id === 'index' ? 'index.md' : `${id.replace(/\/index$/, '')}.md`);
		if (!existsSync(mdFile)) continue; // reported by checkPages
		const markdown = read(mdFile);
		const text = prose(markdown);
		if (/^import\s.+\sfrom\s/m.test(text)) problems.push(`${rel(mdFile)}: contains an ESM import (raw MDX fallback?)`);
		const jsx = /^\s*<([A-Z][A-Za-z]*)[\s/>]/m.exec(text);
		if (jsx) problems.push(`${rel(mdFile)}: contains <${jsx[1]}> (raw MDX fallback?)`);
		const snippets = (read(source).match(/^::snippet\{/gm) ?? []).length;
		if (fenceCount(markdown) < snippets) {
			problems.push(`${rel(mdFile)}: ${snippets} ::snippet directives but only ${fenceCount(markdown)} code blocks`);
		}
	}
	return problems;
}

function checkLlmsFiles(): Problems {
	const problems: Problems = [];
	for (const name of ['llms.txt', 'llms-full.txt', 'llms-small.txt']) {
		if (!existsSync(path.join(dist, name))) problems.push(`missing ${name}`);
	}
	if (problems.length) return problems;
	if (!read(path.join(dist, 'llms-full.txt')).includes('```rust')) {
		problems.push('llms-full.txt contains no ```rust code block');
	}
	const smallSize = statSync(path.join(dist, 'llms-small.txt')).size;
	if (smallSize > LLMS_SMALL_LIMIT) {
		problems.push(`llms-small.txt is ${Math.round(smallSize / 1024)} KB; the limit is ${LLMS_SMALL_LIMIT / 1024} KB`);
	}
	return problems;
}

function checkLegacyUrls(): Problems {
	const problems: Problems = [];
	const urls = read(path.join(projectRoot, 'scripts/legacy-urls.txt'))
		.split('\n')
		.map((line) => line.trim())
		.filter((line) => line && !line.startsWith('#'));
	for (const url of urls) {
		const file = htmlFor(url);
		if (!existsSync(file)) {
			problems.push(`legacy URL ${url} is not served`);
			continue;
		}
		const target = REDIRECT.exec(read(file))?.[1];
		if (target === undefined) continue;
		const targetFile = htmlFor(new URL(target, SITE).pathname);
		if (!existsSync(targetFile)) problems.push(`legacy URL ${url} redirects to missing ${target}`);
		else if (isRedirect(read(targetFile))) problems.push(`legacy URL ${url} redirects to another redirect (${target})`);
	}
	return problems;
}

function main(): void {
	if (!existsSync(dist)) {
		console.error(`${dist} does not exist; run \`pnpm build\` first.`);
		process.exit(1);
	}
	const html = filesUnder(dist, ['.html']).filter((f) => !rel(f).startsWith('pagefind'));
	const text = filesUnder(dist, ['.md', '.txt']);
	const checks: [string, Problems][] = [
		['pages have markdown siblings', checkPages(html)],
		['no unexpanded directives', checkLeftovers([...html, ...text])],
		['markdown siblings are complete', checkMarkdownSiblings()],
		['llms files', checkLlmsFiles()],
		['legacy URLs resolve', checkLegacyUrls()],
	];
	let failed = false;
	for (const [name, problems] of checks) {
		console.log(`${problems.length ? 'FAIL' : 'ok  '} ${name}`);
		for (const problem of problems) console.log(`     ${problem}`);
		failed ||= problems.length > 0;
	}
	if (failed) process.exit(1);
}

main();
