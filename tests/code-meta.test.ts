import { describe, expect, it } from 'vitest';
import { codeMeta, parseCodeDisplay } from '../src/lib/remark/code-meta.ts';

describe('parseCodeDisplay', () => {
	it('normalizes ranges', () => {
		expect(parseCodeDisplay({ collapse: '1-3', mark: '4, 6-7' }, 10)).toMatchObject({ collapse: '1-3', mark: '4,6-7' });
	});
	it('rejects ranges outside the excerpt', () => {
		expect(() => parseCodeDisplay({ mark: '9-12' }, 10)).toThrow(/outside the excerpt \(1-10\)/);
	});
	it('rejects malformed ranges and frames', () => {
		expect(() => parseCodeDisplay({ ins: 'a-b' }, 10)).toThrow(/line ranges/);
		expect(() => parseCodeDisplay({ frame: 'window' }, 10)).toThrow(/frame/);
		expect(() => parseCodeDisplay({ title: 'a "b"' }, 10)).toThrow(/double quotes/);
	});
});

describe('codeMeta', () => {
	const display = parseCodeDisplay(
		{ title: 'src/main.rs', collapse: '1-2', mark: '3', ins: '4', del: '5', frame: 'code', wrap: '' },
		10,
	);
	it('renders Expressive Code meta for html', () => {
		expect(codeMeta(display, 'html')).toBe('title="src/main.rs" frame="code" collapse={1-2} {3} ins={4} del={5} wrap');
	});
	it('keeps only the title for markdown', () => {
		expect(codeMeta(display, 'markdown')).toBe('title="src/main.rs"');
	});
	it('is empty without options', () => {
		expect(codeMeta({}, 'html')).toBeUndefined();
	});
});
