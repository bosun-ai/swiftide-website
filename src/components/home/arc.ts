/**
 * The page's one motif: the arc of the tide in swiftide's mark, the crest the rocket rides
 * (`src/assets/mark.svg`, `A8.6 8.6 0 1 1`). In the mark it is a circle of radius 8.6 drawn from
 * its lowest point, clockwise over the top, to just above its right-hand side: 262 degrees, open
 * at the lower right where the wake runs out.
 *
 * Here it is centred on the origin in a 20 x 20 box, so it can be placed, rotated and scaled.
 */

export const ARC_VIEWBOX = '-10 -10 20 20';

/** Same radius, sweep and opening as the mark's arc. */
export const ARC_PATH = 'M0 8.6A8.6 8.6 0 1 1 8.52 -1.2';

/** Stroke width of the mark, in arc units. */
export const ARC_STROKE = 2.6;

/** The arc as a standalone SVG for CSS masks (`mask-image`), stroked in black. */
export function arcMaskUrl(): string {
	const svg = `<svg xmlns='http://www.w3.org/2000/svg' viewBox='${ARC_VIEWBOX}'><path d='${ARC_PATH}' fill='none' stroke='black' stroke-width='${ARC_STROKE}' stroke-linecap='round'/></svg>`;
	return `url("data:image/svg+xml,${encodeURIComponent(svg)}")`;
}
