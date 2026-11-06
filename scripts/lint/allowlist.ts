/**
 * `scripts/lint-identifiers.allow.txt`: identifiers the lint accepts although swiftide does not
 * define them. One per line, `identifier  # reason`; the reason is mandatory, and entries that
 * no page needs any more are reported as stale.
 */
export interface AllowEntry {
	readonly identifier: string;
	readonly reason: string;
	readonly line: number;
}

export function parseAllowlist(text: string): { entries: AllowEntry[]; errors: string[] } {
	const entries: AllowEntry[] = [];
	const errors: string[] = [];
	text.split('\n').forEach((raw, index) => {
		const line = raw.trim();
		if (!line || line.startsWith('#')) return;
		const [identifier = '', ...rest] = line.split('#');
		const reason = rest.join('#').trim();
		if (!reason) errors.push(`line ${index + 1}: "${identifier.trim()}" needs a "# reason"`);
		else entries.push({ identifier: identifier.trim(), reason, line: index + 1 });
	});
	return { entries, errors };
}
