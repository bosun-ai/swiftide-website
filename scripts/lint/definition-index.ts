/**
 * Names defined in the pinned swiftide source, built with one ripgrep pass per pattern set.
 *
 * `defined` holds item names (fns, types, modules, consts, macros, enum variants, struct fields,
 * derive_builder builders and setters, `use … as` aliases, names in `pub use` re-exports, crate
 * names). `words` holds every
 * identifier-like token in the source; a name that is only in `words` is used but not defined in
 * swiftide (e.g. a re-exported external type) and is reported as a warning, not an error.
 */
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { rgPath } from '@vscode/ripgrep';

export interface DefinitionIndex {
	readonly defined: ReadonlySet<string>;
	readonly words: ReadonlySet<string>;
	/** Crate roots whose paths are not checked: workspace dependencies plus std/core/alloc. */
	readonly externalRoots: ReadonlySet<string>;
}

const IDENT = '[A-Za-z_][A-Za-z0-9_]*';

const DEFINITIONS = [
	`\\b(?:fn|struct|enum|trait|type|mod|const|static|union)\\s+${IDENT}`,
	`macro_rules!\\s*${IDENT}`,
	`\\bas\\s+${IDENT}`,
	`proc_macro_derive\\(\\s*${IDENT}`,
	// Enum variants and struct fields (and, loosely, a few other things): an identifier that
	// starts a line and is followed by `:` (not `::`), `,`, `(` or `{`.
	`^\\s*(?:pub(?:\\([^)]*\\))?\\s+)?${IDENT}\\s*(?:[,({]|:[^:])`,
];

/** Public re-exports (`pub use a::{B, c::D};`) are part of the API; every name in them counts. */
const REEXPORT = `\\bpub(?:\\([^)]*\\))?\\s+use\\s+[^;]+;`;

const BUILDER = `#\\[derive\\([^)]*\\bBuilder\\b[^)]*\\)\\](?:\\s*(?:#\\[[^\\]]*\\]|//[^\\n]*))*\\s*pub(?:\\([^)]*\\))?\\s+struct\\s+${IDENT}`;

function rg(args: string[], cwd: string): string[] {
	const output = execFileSync(rgPath, ['--no-filename', '--no-line-number', '--no-heading', '-o', '-g', '*.rs', ...args, '.'], {
		cwd,
		encoding: 'utf8',
		maxBuffer: 512 * 1024 * 1024,
	});
	return output.split('\n').filter(Boolean);
}

const lastIdent = (match: string) => match.match(new RegExp(IDENT, 'g'))?.at(-1);

function workspaceDependencies(swiftideDir: string): string[] {
	const toml = readFileSync(path.join(swiftideDir, 'Cargo.toml'), 'utf8');
	const start = toml.indexOf('[workspace.dependencies]');
	if (start === -1) return [];
	const table = toml.slice(start).split(/\n\[/)[0]!;
	return [...table.matchAll(/^([A-Za-z0-9_-]+)\s*=/gm)].map((m) => m[1]!.replaceAll('-', '_'));
}

export function buildDefinitionIndex(swiftideDir: string, crates: readonly string[]): DefinitionIndex {
	const defined = new Set<string>(crates);
	for (const match of rg(DEFINITIONS.flatMap((p) => ['-e', p]), swiftideDir)) {
		const name = lastIdent(match.replace(/(?:[,({]|:[^:]?)$/, ''));
		if (name) defined.add(name);
	}
	for (const match of rg(['-U', '-e', BUILDER], swiftideDir)) {
		const name = lastIdent(match);
		if (name) defined.add(`${name}Builder`).add(`${name}BuilderError`);
	}
	for (const match of rg(['-U', '-e', REEXPORT], swiftideDir)) {
		for (const name of match.slice(match.indexOf('use') + 3).match(new RegExp(IDENT, 'g')) ?? []) {
			if (name !== 'as' && name !== 'self' && name !== 'super' && name !== 'crate') defined.add(name);
		}
	}
	const words = new Set(rg(['-e', `\\b${IDENT}\\b`], swiftideDir));
	const externalRoots = new Set(['std', 'core', 'alloc', ...workspaceDependencies(swiftideDir)]);
	return { defined, words, externalRoots };
}
