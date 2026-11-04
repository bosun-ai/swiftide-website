/** Small builders for the mdast nodes the directive expanders emit. */
import type { Link, Paragraph, PhrasingContent, Root, Table, TableCell, TableRow, Text } from 'mdast';
// Augments mdast `data` with `hName` / `hProperties`.
import type {} from 'mdast-util-to-hast';
import remarkGfm from 'remark-gfm';
import remarkParse from 'remark-parse';
import { unified } from 'unified';

export const text = (value: string): Text => ({ type: 'text', value });

export const code = (value: string): PhrasingContent => ({ type: 'inlineCode', value });

export const link = (url: string, children: PhrasingContent[]): Link => ({ type: 'link', url, children });

export type Cell = PhrasingContent[];

const cell = (children: Cell): TableCell => ({ type: 'tableCell', children });
const row = (cells: readonly Cell[]): TableRow => ({ type: 'tableRow', children: cells.map(cell) });

export function table(header: readonly string[], rows: readonly (readonly Cell[])[]): Table {
	return {
		type: 'table',
		align: header.map(() => null),
		children: [row(header.map((h) => [text(h)])), ...rows.map(row)],
	};
}

/** Joins phrasing lists with a separator, e.g. several links with ", ". */
export function joined(parts: readonly Cell[], separator = ', '): Cell {
	return parts.flatMap((part, index) => (index === 0 ? part : [text(separator), ...part]));
}

const markdownParser = unified().use(remarkParse).use(remarkGfm);

/** Parses a markdown document (CommonMark + GFM) into mdast. */
export function parseMarkdown(markdown: string): Root {
	return markdownParser.parse(markdown);
}

/** Parses a single line of markdown into phrasing content. */
export function parseInline(markdown: string): Cell {
	const first = parseMarkdown(markdown).children[0];
	return first?.type === 'paragraph' ? (first as Paragraph).children : markdown ? [text(markdown)] : [];
}
