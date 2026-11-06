import { readFileSync } from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import {
	cutExcerpt,
	dedent,
	languageFor,
	listAnchors,
	loadExcerpt,
	parseSelector,
	resolveSource,
	sourceRoots,
} from '../src/lib/snippets.ts';
import { FIXTURE_ROOT, fixtureSwiftide } from './helpers.ts';

const roots = sourceRoots(FIXTURE_ROOT, fixtureSwiftide);
const agents = readFileSync(path.join(FIXTURE_ROOT, 'snippets/src/agents.rs'), 'utf8');
const anchor = (name: string) => parseSelector(name, undefined);

describe('cutExcerpt', () => {
	it('cuts an anchor and strips nested markers', () => {
		const excerpt = cutExcerpt(agents, anchor('outer'), 'agents.rs');
		expect(excerpt.code).toBe('pub fn outer() {\n    let a = 1;\n    let b = 2;\n    println!("{a} {b}");\n}');
		expect(excerpt).toMatchObject({ startLine: 4, endLine: 10 });
	});

	it('cuts a nested anchor on its own', () => {
		expect(cutExcerpt(agents, anchor('inner'), 'agents.rs')).toEqual({ code: 'let b = 2;', startLine: 7, endLine: 7 });
	});

	it('dedents indented anchors', () => {
		const { code } = cutExcerpt(agents, anchor('indented'), 'agents.rs');
		expect(code.split('\n')[0]).toBe("fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {");
		expect(code.split('\n')[1]).toBe('    write!(f, "thing")');
	});

	it('reports missing anchors with the available ones', () => {
		expect(() => cutExcerpt(agents, anchor('nope'), 'agents.rs')).toThrow(
			/Anchor "nope" not found in agents.rs. Available anchors: outer, inner, indented, duplicated, duplicated, unterminated/,
		);
	});

	it('rejects duplicate anchors', () => {
		expect(() => cutExcerpt(agents, anchor('duplicated'), 'agents.rs')).toThrow(/defined 2 times/);
	});

	it('rejects unterminated anchors', () => {
		expect(() => cutExcerpt(agents, anchor('unterminated'), 'agents.rs')).toThrow(/no matching ANCHOR_END/);
	});

	it('cuts line ranges and still strips markers', () => {
		const excerpt = cutExcerpt(agents, parseSelector(undefined, '5-7'), 'agents.rs');
		expect(excerpt).toEqual({ code: 'let a = 1;\nlet b = 2;', startLine: 5, endLine: 7 });
	});

	it('rejects ranges past the end of the file', () => {
		expect(() => cutExcerpt('a\nb', parseSelector(undefined, '1-3'), 'f')).toThrow(/past the end of f \(2 lines\)/);
	});

	it('selects the whole file without anchor or lines', () => {
		expect(cutExcerpt('x\n// ANCHOR: a\ny\n// ANCHOR_END: a\n', parseSelector(undefined, undefined), 'f').code).toBe('x\ny');
	});
});

describe('parseSelector', () => {
	it('refuses anchor and lines together', () => {
		expect(() => parseSelector('a', '1-2')).toThrow(/either anchor/);
	});
	it('refuses malformed ranges', () => {
		expect(() => parseSelector(undefined, '3-1')).toThrow(/Invalid line range/);
		expect(() => parseSelector(undefined, 'x')).toThrow(/Invalid lines/);
	});
});

describe('dedent', () => {
	it('keeps relative indentation and empties blank lines', () => {
		expect(dedent(['    a', '', '      b', '  '])).toEqual(['a', '', '  b', '']);
	});
});

describe('resolveSource', () => {
	it('resolves aliases', () => {
		expect(resolveSource('@swiftide/examples/hello.rs', roots).relPath).toBe('examples/hello.rs');
		expect(resolveSource('@snippets/src/agents.rs', roots).relPath).toBe('src/agents.rs');
	});
	it('rejects unknown aliases', () => {
		expect(() => resolveSource('src/agents.rs', roots)).toThrow(/must start with one of: @swiftide\/, @snippets\//);
	});
	it('rejects paths escaping the root', () => {
		expect(() => resolveSource('@snippets/../src/data/integrations.yaml', roots)).toThrow(/outside @snippets\//);
	});
	it('rejects missing files', () => {
		expect(() => resolveSource('@snippets/src/missing.rs', roots)).toThrow(/not found/);
	});
});

describe('loadExcerpt', () => {
	it('loads from the swiftide checkout', () => {
		const excerpt = loadExcerpt('@swiftide/examples/hello.rs', anchor('build'), roots);
		expect(excerpt.code).toBe('let agent = Agent::builder().build();');
		expect(excerpt.source.root.permalink(excerpt.source.relPath, { start: 7, end: 7 })).toBe(
			`${fixtureSwiftide.ghBlob}examples/hello.rs#L7-L7`,
		);
	});
});

describe('languageFor / listAnchors', () => {
	it('maps extensions', () => {
		expect(languageFor('a/b.rs')).toBe('rust');
		expect(languageFor('Cargo.toml')).toBe('toml');
		expect(languageFor('prompt.md.tera')).toBe('jinja');
		expect(languageFor('LICENSE')).toBe('text');
	});
	it('lists anchors', () => {
		expect(listAnchors('// ANCHOR: a\n// ANCHOR_END: a\n# ANCHOR: b')).toEqual(['a', 'b']);
	});
});
