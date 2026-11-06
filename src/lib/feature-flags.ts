/**
 * Feature flags of the `swiftide` crate, read from the `[features]` table of its Cargo.toml.
 *
 * Follows the `document-features` conventions swiftide already uses for docs.rs: `## text` lines
 * document the next feature, `#! ### Title` starts a section. A line scanner is enough because
 * the table only ever contains `name = [ "…", … ]` entries.
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';

export interface FeatureFlag {
	readonly name: string;
	/** Markdown from the `##` lines above the feature; empty when undocumented. */
	readonly doc: string;
	/** Other features of the same crate this one turns on. */
	readonly enables: readonly string[];
}

export interface FeatureSection {
	/** `null` for features declared before the first `#! ###` heading. */
	readonly title: string | null;
	readonly flags: readonly FeatureFlag[];
}

interface MutableSection {
	title: string | null;
	flags: FeatureFlag[];
}

const FEATURE_START = /^([A-Za-z0-9_-]+)\s*=\s*\[(.*)$/;

function quoted(text: string): string[] {
	return [...text.matchAll(/"([^"]*)"/g)].map((m) => m[1]!);
}

export function parseFeatureFlags(cargoToml: string): FeatureSection[] {
	const lines = cargoToml.replace(/\r\n/g, '\n').split('\n');
	const start = lines.findIndex((line) => line.trim() === '[features]');
	if (start === -1) throw new Error('Cargo.toml has no [features] table');

	const sections: MutableSection[] = [{ title: null, flags: [] }];
	let docs: string[] = [];
	const raw: { section: MutableSection; name: string; doc: string; entries: string[] }[] = [];

	for (let i = start + 1; i < lines.length; i++) {
		const line = lines[i]!.trim();
		if (line.startsWith('[')) break;
		if (line.startsWith('#!')) {
			const heading = /^#!\s*#+\s*(.+?)\s*$/.exec(line);
			if (heading) sections.push({ title: heading[1]!, flags: [] });
			continue;
		}
		if (line.startsWith('##')) {
			docs.push(line.replace(/^##\s?/, '').trimEnd());
			continue;
		}
		const feature = FEATURE_START.exec(line);
		if (!feature) continue;
		let body = feature[2]!;
		while (!body.includes(']') && i + 1 < lines.length) {
			body += ` ${lines[++i]!.trim()}`;
		}
		raw.push({
			section: sections.at(-1)!,
			name: feature[1]!,
			doc: docs.join(' ').trim(),
			entries: quoted(body.slice(0, body.indexOf(']'))),
		});
		docs = [];
	}

	const names = new Set(raw.map((r) => r.name));
	for (const r of raw) {
		r.section.flags.push({
			name: r.name,
			doc: r.doc,
			enables: r.entries.filter((entry) => names.has(entry)),
		});
	}
	return sections.filter((section) => section.flags.length > 0);
}

/** Feature flags of `vendor/swiftide/swiftide/Cargo.toml`. */
export function loadFeatureFlags(swiftideDir: string): FeatureSection[] {
	return parseFeatureFlags(readFileSync(path.join(swiftideDir, 'swiftide', 'Cargo.toml'), 'utf8'));
}

export function featureNames(sections: readonly FeatureSection[]): Set<string> {
	return new Set(sections.flatMap((section) => section.flags.map((flag) => flag.name)));
}
