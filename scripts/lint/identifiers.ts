/**
 * Classifies inline code from the docs into Rust identifiers and checks them against the
 * definition index. Inline code that does not look like a Rust path, type or method is ignored.
 */
import type { DefinitionIndex } from './definition-index.ts';

export type IdentifierKind = 'path' | 'type' | 'method';

export interface Identifier {
	readonly kind: IdentifierKind;
	/** The normalized identifier, e.g. `swiftide::agents::Agent` or `with_tools`. */
	readonly name: string;
	readonly segments: readonly string[];
}

export type Verdict =
	| { readonly status: 'ok' }
	| { readonly status: 'unknown'; readonly segment: string; readonly suggestion?: string }
	| { readonly status: 'external'; readonly segment: string };

const PATH = /^\w+(?:::\w+)+$/;
const TYPE = /^[A-Z][A-Za-z0-9]*$/;
const METHOD = /^[a-z][a-z0-9]*(?:_[a-z0-9]+)+$/;
const SIMPLE = /^[a-z_][a-z0-9_]*$/;

function stripGenerics(value: string): string {
	let previous: string;
	let current = value;
	do {
		previous = current;
		current = current.replace(/<[^<>]*>/g, '');
	} while (current !== previous);
	return current;
}

/** Returns the identifier written in an inline code span, or `undefined` if it is not one. */
export function classify(raw: string): Identifier | undefined {
	let value = raw.trim().replace(/^&(?:mut\s+)?/, '').replace(/^\./, '').replace(/^#\[|\]$/g, '');
	const called = /\([^()]*\)$/.test(value);
	value = stripGenerics(value.replace(/\([^()]*\)$/, '').replace(/!$/, ''));
	if (value.endsWith('::*')) value = value.slice(0, -3);
	if (PATH.test(value)) return { kind: 'path', name: value, segments: value.split('::') };
	const isType = TYPE.test(value) && (value.match(/[A-Z]/g)?.length ?? 0) >= 2 && /[a-z]/.test(value);
	if (isType) return { kind: 'type', name: value, segments: [value] };
	if (METHOD.test(value) || (called && SIMPLE.test(value))) return { kind: 'method', name: value, segments: [value] };
	return undefined;
}

const PATH_KEYWORDS = new Set(['crate', 'self', 'super', 'Self']);

/** Edit distance counting a swap of two adjacent characters as one edit (typos like "Agnet"). */
function editDistance(a: string, b: string): number {
	const d: number[][] = Array.from({ length: a.length + 1 }, (_, i) =>
		Array.from({ length: b.length + 1 }, (_, j) => (i === 0 ? j : j === 0 ? i : 0)),
	);
	for (let i = 1; i <= a.length; i++) {
		for (let j = 1; j <= b.length; j++) {
			const cost = a[i - 1] === b[j - 1] ? 0 : 1;
			let best = Math.min(d[i - 1]![j]! + 1, d[i]![j - 1]! + 1, d[i - 1]![j - 1]! + cost);
			if (i > 1 && j > 1 && a[i - 1] === b[j - 2] && a[i - 2] === b[j - 1]) best = Math.min(best, d[i - 2]![j - 2]! + 1);
			d[i]![j] = best;
		}
	}
	return d[a.length]![b.length]!;
}

/** The closest defined name, if one is close enough to be a likely typo. */
export function suggest(name: string, defined: ReadonlySet<string>): string | undefined {
	const limit = Math.max(2, Math.floor(name.length / 4));
	let best: { name: string; distance: number } | undefined;
	for (const candidate of defined) {
		if (Math.abs(candidate.length - name.length) > limit) continue;
		const distance = editDistance(name, candidate);
		if (distance <= limit && (!best || distance < best.distance)) best = { name: candidate, distance };
	}
	return best?.name;
}

export function check(identifier: Identifier, index: DefinitionIndex): Verdict {
	const [root] = identifier.segments;
	if (identifier.kind === 'path' && root && index.externalRoots.has(root)) return { status: 'ok' };
	for (const segment of identifier.segments) {
		if (PATH_KEYWORDS.has(segment) || index.defined.has(segment)) continue;
		if (index.words.has(segment)) return { status: 'external', segment };
		const suggestion = suggest(segment, index.defined);
		return suggestion ? { status: 'unknown', segment, suggestion } : { status: 'unknown', segment };
	}
	return { status: 'ok' };
}
