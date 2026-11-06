/**
 * Resolves `::snippet` sources and cuts excerpts out of them.
 *
 * Pure with respect to markdown: this module knows about files, anchors and line ranges, never
 * about mdast. Every failure throws a `SnippetError`; a page never renders a placeholder.
 */
import { readFileSync, realpathSync } from 'node:fs';
import path from 'node:path';
import { swiftidePermalink, type SwiftideContext } from './swiftide.ts';

export const WEBSITE_REPO = 'https://github.com/bosun-ai/swiftide-website';

export class SnippetError extends Error {
	constructor(message: string) {
		super(message);
		this.name = 'SnippetError';
	}
}

/** A directory that `src` attributes may point into, addressed through an alias. */
export interface SourceRoot {
	readonly alias: string;
	/** Real (symlink-resolved) absolute directory. */
	readonly dir: string;
	/** How a path relative to `dir` is shown to readers, e.g. `examples/hello_agents.rs`. */
	label(relPath: string): string;
	/** Permalink for a path relative to `dir`. */
	permalink(relPath: string, lines?: { start: number; end: number }): string;
}

export interface ResolvedSource {
	readonly root: SourceRoot;
	readonly absPath: string;
	/** Path relative to the root, with forward slashes. */
	readonly relPath: string;
}

export interface Excerpt {
	readonly source: ResolvedSource;
	readonly code: string;
	/** 1-based, inclusive line numbers of the excerpt in the original file. */
	readonly startLine: number;
	readonly endLine: number;
	readonly lineCount: number;
}

export type Selector =
	| { readonly kind: 'anchor'; readonly name: string }
	| { readonly kind: 'lines'; readonly start: number; readonly end: number }
	| { readonly kind: 'file' };

/** The aliases pages may use: `@swiftide/` for the pinned checkout, `@snippets/` for our crate. */
export function sourceRoots(projectRoot: string, swiftide: SwiftideContext): SourceRoot[] {
	return [
		{
			alias: '@swiftide/',
			dir: realpathSync(swiftide.dir),
			label: (relPath) => relPath,
			permalink: (relPath, lines) => swiftidePermalink(swiftide, relPath, lines),
		},
		{
			alias: '@snippets/',
			dir: realpathSync(path.resolve(projectRoot, 'snippets')),
			label: (relPath) => `snippets/${relPath}`,
			permalink: (relPath, lines) =>
				`${WEBSITE_REPO}/blob/master/snippets/${relPath}${lines ? `#L${lines.start}-L${lines.end}` : ''}`,
		},
	];
}

/** Maps an aliased `src` to a file inside its root. Paths escaping the root are rejected. */
export function resolveSource(src: string, roots: readonly SourceRoot[]): ResolvedSource {
	const root = roots.find((candidate) => src.startsWith(candidate.alias));
	if (!root) {
		const aliases = roots.map((r) => r.alias).join(', ');
		throw new SnippetError(`Snippet src "${src}" must start with one of: ${aliases}`);
	}
	const requested = path.resolve(root.dir, src.slice(root.alias.length));
	let absPath: string;
	try {
		absPath = realpathSync(requested);
	} catch {
		throw new SnippetError(`Snippet source not found: ${src} (looked at ${requested})`);
	}
	const relPath = path.relative(root.dir, absPath);
	if (relPath.startsWith('..') || path.isAbsolute(relPath)) {
		throw new SnippetError(`Snippet src "${src}" resolves outside ${root.alias}: ${absPath}`);
	}
	return { root, absPath, relPath: relPath.split(path.sep).join('/') };
}

const MARKER = /\bANCHOR(_END)?:\s*([A-Za-z0-9_-]+)/;

function isMarker(line: string): boolean {
	return MARKER.test(line);
}

/** Names of every `ANCHOR:` in a file, in order of appearance. */
export function listAnchors(text: string): string[] {
	return text
		.split('\n')
		.map((line) => MARKER.exec(line))
		.filter((m): m is RegExpExecArray => m !== null && m[1] === undefined)
		.map((m) => m[2]!);
}

function findMarkers(lines: readonly string[], name: string, end: boolean): number[] {
	const found: number[] = [];
	lines.forEach((line, index) => {
		const m = MARKER.exec(line);
		if (m && m[2] === name && (m[1] !== undefined) === end) found.push(index);
	});
	return found;
}

/** Removes the common leading indentation of all non-blank lines. */
export function dedent(lines: readonly string[]): string[] {
	const indents = lines
		.filter((line) => line.trim() !== '')
		.map((line) => line.length - line.trimStart().length);
	const common = indents.length ? Math.min(...indents) : 0;
	return lines.map((line) => (line.trim() === '' ? '' : line.slice(common)));
}

/**
 * Parses a selector from directive attributes. Exactly one of `anchor` / `lines` may be given;
 * neither selects the whole file.
 */
export function parseSelector(anchor: string | undefined, lines: string | undefined): Selector {
	if (anchor !== undefined && lines !== undefined) {
		throw new SnippetError('Use either anchor="…" or lines="…", not both');
	}
	if (anchor !== undefined) {
		if (!/^[A-Za-z0-9_-]+$/.test(anchor)) throw new SnippetError(`Invalid anchor name "${anchor}"`);
		return { kind: 'anchor', name: anchor };
	}
	if (lines !== undefined) {
		const m = /^(\d+)(?:-(\d+))?$/.exec(lines.trim());
		if (!m) throw new SnippetError(`Invalid lines="${lines}"; expected "start-end" (1-based, inclusive)`);
		const start = Number(m[1]);
		const end = m[2] === undefined ? start : Number(m[2]);
		if (start < 1 || end < start) throw new SnippetError(`Invalid line range lines="${lines}"`);
		return { kind: 'lines', start, end };
	}
	return { kind: 'file' };
}

/** Cuts the selected region out of `text`, strips nested anchor markers and dedents it. */
export function cutExcerpt(
	text: string,
	selector: Selector,
	label: string,
): { code: string; startLine: number; endLine: number } {
	const lines = text.replace(/\r\n/g, '\n').replace(/\n$/, '').split('\n');
	let from: number; // 0-based inclusive
	let to: number; // 0-based exclusive
	switch (selector.kind) {
		case 'anchor': {
			const starts = findMarkers(lines, selector.name, false);
			const ends = findMarkers(lines, selector.name, true);
			const available = listAnchors(text);
			if (starts.length === 0) {
				throw new SnippetError(
					`Anchor "${selector.name}" not found in ${label}. ` +
						(available.length ? `Available anchors: ${available.join(', ')}` : 'The file has no anchors.'),
				);
			}
			if (starts.length > 1) {
				throw new SnippetError(
					`Anchor "${selector.name}" is defined ${starts.length} times in ${label} (lines ${starts.map((i) => i + 1).join(', ')})`,
				);
			}
			const start = starts[0]!;
			const end = ends.find((index) => index > start);
			if (end === undefined) {
				throw new SnippetError(`Anchor "${selector.name}" in ${label} has no matching ANCHOR_END`);
			}
			if (ends.length > 1) {
				throw new SnippetError(`Anchor "${selector.name}" in ${label} has ${ends.length} ANCHOR_END markers`);
			}
			from = start + 1;
			to = end;
			break;
		}
		case 'lines': {
			if (selector.end > lines.length) {
				throw new SnippetError(
					`lines="${selector.start}-${selector.end}" is past the end of ${label} (${lines.length} lines)`,
				);
			}
			from = selector.start - 1;
			to = selector.end;
			break;
		}
		case 'file':
			from = 0;
			to = lines.length;
	}
	const body = dedent(lines.slice(from, to).filter((line) => !isMarker(line)));
	if (body.every((line) => line.trim() === '')) {
		throw new SnippetError(`The excerpt selected from ${label} is empty`);
	}
	return { code: body.join('\n'), startLine: from + 1, endLine: to };
}

/** Resolves `src`, reads it and cuts the excerpt. */
export function loadExcerpt(src: string, selector: Selector, roots: readonly SourceRoot[]): Excerpt {
	const source = resolveSource(src, roots);
	const text = readFileSync(source.absPath, 'utf8');
	const { code, startLine, endLine } = cutExcerpt(text, selector, src);
	return { source, code, startLine, endLine, lineCount: code.split('\n').length };
}

const LANGUAGES: Record<string, string> = {
	rs: 'rust',
	toml: 'toml',
	md: 'md',
	sh: 'sh',
	bash: 'bash',
	yaml: 'yaml',
	yml: 'yaml',
	json: 'json',
	ts: 'ts',
	js: 'js',
	py: 'python',
	sql: 'sql',
	txt: 'text',
	j2: 'jinja',
	jinja: 'jinja',
	tera: 'jinja',
	md_tera: 'jinja',
};

/** Code fence language for a file, from its extension. */
export function languageFor(relPath: string): string {
	const base = path.basename(relPath);
	const ext = base.includes('.') ? base.slice(base.indexOf('.') + 1).replaceAll('.', '_') : '';
	return LANGUAGES[ext] ?? LANGUAGES[ext.split('_').pop() ?? ''] ?? 'text';
}
