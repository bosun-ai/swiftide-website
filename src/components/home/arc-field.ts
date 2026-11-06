/**
 * The "many into one" drawing for the how-it-works band, built only from the motif arc.
 *
 * Three zones, read left to right: values scattered at random angles in two muted tones (the
 * clients, executors and stores you build), a ribbon three arcs high (the few traits they share),
 * and evenly spaced rows in the accent (the three runtimes that take them). The ribbon and the
 * rows all face one way, so order reads against the scatter. Each zone is its own SVG so the text
 * under it can sit level with it at every width. Positions come from a seeded generator, so the
 * drawing is the same on every build. One user unit is about one pixel at 1440px.
 */

export interface PlacedArc {
	readonly x: number;
	readonly y: number;
	/** Rendered size in user units (the arc's 20-unit box scaled to this). */
	readonly size: number;
	readonly rotate: number;
	readonly tone: 'quiet' | 'soft' | 'ink' | 'accent';
}

export interface ArcZone {
	readonly name: 'values' | 'traits' | 'runtimes';
	readonly width: number;
	readonly height: number;
	readonly arcs: readonly PlacedArc[];
}

export const ZONE_WIDTH = 450;
export const ZONE_HEIGHT = 300;
const MID = ZONE_HEIGHT / 2;
const ARC_SIZE = 18;

/** mulberry32: small, fast and deterministic. */
function random(seed: number): () => number {
	let state = seed >>> 0;
	return () => {
		state = (state + 0x6d2b79f5) >>> 0;
		let t = state;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

/** Loose at the left edge, drawn together towards the ribbon on the right. */
function scattered(count: number, next: () => number): PlacedArc[] {
	return Array.from({ length: count }, () => {
		const along = Math.sqrt(next());
		const spread = 1 - 0.5 * along;
		return {
			x: 30 + along * (ZONE_WIDTH - 60),
			y: MID + (next() - 0.5) * (ZONE_HEIGHT - 70) * spread,
			size: 16 + next() * 4,
			rotate: Math.round(next() * 360),
			tone: next() < 0.5 ? 'quiet' : 'soft',
		};
	});
}

/** A grid of `perRow` x 3 arcs, one orientation, centred on the zone. */
function grid(perRow: number, columnGap: number, rowGap: number, tone: PlacedArc['tone']): PlacedArc[] {
	const left = (ZONE_WIDTH - columnGap * (perRow - 1)) / 2;
	return [-1, 0, 1].flatMap((row) =>
		Array.from({ length: perRow }, (_, column) => ({
			x: left + columnGap * column,
			y: MID + row * rowGap,
			size: ARC_SIZE,
			rotate: 0,
			tone,
		})),
	);
}

export function arcField(): readonly ArcZone[] {
	const next = random(609723331);
	const zone = (name: ArcZone['name'], arcs: PlacedArc[]): ArcZone => ({
		name,
		width: ZONE_WIDTH,
		height: ZONE_HEIGHT,
		arcs,
	});
	// Ribbon: arcs half their own size apart, rows close. Rows: wide apart, one per runtime.
	return [
		zone('values', scattered(28, next)),
		zone('traits', grid(10, ARC_SIZE * 1.6, ARC_SIZE * 1.5, 'ink')),
		zone('runtimes', grid(7, 60, 72, 'accent')),
	];
}
