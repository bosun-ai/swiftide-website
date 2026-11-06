/** Attribute validation shared by all directives: unknown attributes are typos, and fail. */
import { SnippetError } from '../../snippets.ts';

/** Directive attributes as remark-directive produces them. */
export type Attributes = Readonly<Record<string, string | null | undefined>>;

/** Validated attributes: only allowed keys, `null` normalized to `undefined`. */
export type CheckedAttributes = Readonly<Record<string, string | undefined>>;

export function checkAttributes(
	directive: string,
	attributes: Attributes | null | undefined,
	allowed: readonly string[],
	required: readonly string[] = [],
): CheckedAttributes {
	const attrs: Record<string, string | undefined> = {};
	for (const [key, value] of Object.entries(attributes ?? {})) attrs[key] = value ?? undefined;
	for (const key of Object.keys(attrs)) {
		if (!allowed.includes(key)) {
			throw new SnippetError(
				`Unknown attribute "${key}" on ::${directive}. Allowed: ${allowed.join(', ') || '(none)'}`,
			);
		}
	}
	for (const key of required) {
		if (!attrs[key]) throw new SnippetError(`::${directive} needs a ${key}="…" attribute`);
	}
	return attrs;
}
