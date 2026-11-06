/**
 * Display options of a code block and their rendering to an Expressive Code meta string.
 *
 * Ranges are relative to the excerpt (line 1 is the first line shown), validated against its
 * length so a stale `mark="…"` fails the build instead of silently highlighting nothing.
 */
import { SnippetError } from '../snippets.ts';

export type RenderTarget = 'html' | 'markdown';

export interface CodeDisplay {
	readonly title?: string | undefined;
	readonly collapse?: string | undefined;
	readonly mark?: string | undefined;
	readonly ins?: string | undefined;
	readonly del?: string | undefined;
	readonly frame?: 'code' | 'terminal' | 'none' | 'auto' | undefined;
	readonly wrap?: boolean | undefined;
}

const RANGES = /^\d+(?:-\d+)?(?:\s*,\s*\d+(?:-\d+)?)*$/;
const FRAMES = new Set(['code', 'terminal', 'none', 'auto']);

function checkRanges(name: string, value: string, lineCount: number): string {
	const normalized = value.replace(/\s+/g, '');
	if (!RANGES.test(normalized)) {
		throw new SnippetError(`${name}="${value}" must be line ranges like "1-6" or "3,8-10"`);
	}
	for (const part of normalized.split(',')) {
		const [start, end = start] = part.split('-').map(Number) as [number, number?];
		if (start < 1 || end! < start || end! > lineCount) {
			throw new SnippetError(`${name}="${value}" is outside the excerpt (1-${lineCount})`);
		}
	}
	return normalized;
}

/** Validates raw directive attributes into display options. */
export function parseCodeDisplay(
	attributes: Readonly<Record<string, string | undefined>>,
	lineCount: number,
): CodeDisplay {
	const { title, collapse, mark, ins, del, frame, wrap } = attributes;
	if (title !== undefined && title.includes('"')) {
		throw new SnippetError(`title="${title}" must not contain double quotes`);
	}
	if (frame !== undefined && !FRAMES.has(frame)) {
		throw new SnippetError(`frame="${frame}" must be one of ${[...FRAMES].join(', ')}`);
	}
	if (wrap !== undefined && wrap !== '' && wrap !== 'true' && wrap !== 'false') {
		throw new SnippetError(`wrap="${wrap}" must be a boolean`);
	}
	return {
		title: title || undefined,
		collapse: collapse === undefined ? undefined : checkRanges('collapse', collapse, lineCount),
		mark: mark === undefined ? undefined : checkRanges('mark', mark, lineCount),
		ins: ins === undefined ? undefined : checkRanges('ins', ins, lineCount),
		del: del === undefined ? undefined : checkRanges('del', del, lineCount),
		frame: frame as CodeDisplay['frame'],
		wrap: wrap === undefined ? undefined : wrap !== 'false',
	};
}

/**
 * The fence meta for a code block. HTML gets everything Expressive Code understands; markdown
 * keeps only the title, which is meaningful to a reader without a renderer.
 */
export function codeMeta(display: CodeDisplay, target: RenderTarget): string | undefined {
	const parts: string[] = [];
	if (display.title) parts.push(`title="${display.title}"`);
	if (target === 'html') {
		if (display.frame) parts.push(`frame="${display.frame}"`);
		if (display.collapse) parts.push(`collapse={${display.collapse}}`);
		if (display.mark) parts.push(`{${display.mark}}`);
		if (display.ins) parts.push(`ins={${display.ins}}`);
		if (display.del) parts.push(`del={${display.del}}`);
		if (display.wrap) parts.push('wrap');
	}
	return parts.length ? parts.join(' ') : undefined;
}
