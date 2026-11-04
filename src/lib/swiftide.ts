/**
 * Version context of the pinned swiftide checkout.
 *
 * The superproject's gitlink for `vendor/swiftide` is the single source of truth for which
 * swiftide the site documents. Everything that mentions a version, a docs.rs URL or a GitHub
 * permalink derives it from here. Missing tags or a checkout that does not match the gitlink are
 * errors: the site must never be built against an unknown swiftide.
 */
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import path from 'node:path';

export const SWIFTIDE_REPO = 'https://github.com/bosun-ai/swiftide';
export const VENDOR_DIR = 'vendor/swiftide';

export interface SwiftideContext {
	/** Absolute path to the swiftide checkout. */
	readonly dir: string;
	/** Full commit SHA recorded by the gitlink. */
	readonly sha: string;
	readonly shortSha: string;
	/** Latest release tag reachable from `sha`, e.g. `v0.33.0`. */
	readonly tag: string;
	/** `tag` without the `v`, e.g. `0.33.0`. */
	readonly version: string;
	/** Commits on top of `tag`. Non-zero means the site documents unreleased code. */
	readonly commitsAhead: number;
	/** Crate names (underscored) that live in the swiftide workspace, e.g. `swiftide_core`. */
	readonly crates: readonly string[];
	/** `https://docs.rs/swiftide/<version>/swiftide/` */
	readonly docsRsBase: string;
	/** `https://github.com/bosun-ai/swiftide/blob/<sha>/` */
	readonly ghBlob: string;
}

export class SwiftideContextError extends Error {
	constructor(message: string) {
		super(message);
		this.name = 'SwiftideContextError';
	}
}

const DESCRIBE = /^v(\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?)-(\d+)-g([0-9a-f]+)$/;

function git(cwd: string, args: string[]): string {
	return execFileSync('git', ['-C', cwd, ...args], {
		encoding: 'utf8',
		stdio: ['ignore', 'pipe', 'pipe'],
	}).trim();
}

function gitlinkSha(projectRoot: string): string {
	const entry = git(projectRoot, ['ls-files', '--stage', '--', VENDOR_DIR]);
	const [mode, sha] = entry.split(/\s+/);
	if (mode !== '160000' || !sha) {
		throw new SwiftideContextError(
			`${VENDOR_DIR} is not a git submodule of ${projectRoot}. Run scripts/setup.sh.`,
		);
	}
	return sha;
}

function describe(dir: string, sha: string): { tag: string; version: string; commitsAhead: number } {
	let described: string;
	try {
		described = git(dir, ['describe', '--tags', '--long', '--match', 'v[0-9]*', sha]);
	} catch (error) {
		throw new SwiftideContextError(
			`Cannot derive the swiftide version: \`git describe --tags\` failed in ${VENDOR_DIR}. ` +
				'Fetch the tags (`git -C vendor/swiftide fetch --tags`; in CI check out with ' +
				'`fetch-depth: 0`).\n' +
				String(error),
		);
	}
	const match = DESCRIBE.exec(described);
	if (!match) {
		throw new SwiftideContextError(`Unexpected \`git describe\` output in ${VENDOR_DIR}: ${described}`);
	}
	const [, version, ahead] = match;
	return { tag: `v${version}`, version: version!, commitsAhead: Number(ahead) };
}

function workspaceCrates(dir: string): string[] {
	return readdirSync(dir, { withFileTypes: true })
		.filter((entry) => entry.isDirectory() && /^swiftide(-[a-z-]+)?$/.test(entry.name))
		.filter((entry) => existsSync(path.join(dir, entry.name, 'Cargo.toml')))
		.map((entry) => entry.name.replaceAll('-', '_'))
		.sort();
}

/** Reads the pinned swiftide version context. Throws when the checkout cannot be trusted. */
export function loadSwiftideContext(projectRoot: string): SwiftideContext {
	const dir = path.resolve(projectRoot, VENDOR_DIR);
	if (!existsSync(path.join(dir, 'Cargo.toml'))) {
		throw new SwiftideContextError(
			`${VENDOR_DIR} is empty. Run \`git submodule update --init\` or scripts/setup.sh.`,
		);
	}
	const sha = gitlinkSha(projectRoot);
	const head = git(dir, ['rev-parse', 'HEAD']);
	if (head !== sha) {
		throw new SwiftideContextError(
			`${VENDOR_DIR} is checked out at ${head.slice(0, 9)} but the site pins ${sha.slice(0, 9)}. ` +
				'Run `git submodule update` to go back to the pin, or `git add vendor/swiftide` to move it.',
		);
	}
	const { tag, version, commitsAhead } = describe(dir, sha);
	return {
		dir,
		sha,
		shortSha: sha.slice(0, 9),
		tag,
		version,
		commitsAhead,
		crates: workspaceCrates(dir),
		docsRsBase: `https://docs.rs/swiftide/${version}/swiftide/`,
		ghBlob: `${SWIFTIDE_REPO}/blob/${sha}/`,
	};
}

/**
 * Resolves a `docsrs:` target such as `swiftide/agents/struct.Agent.html` or `swiftide_core` to
 * the docs.rs URL of the pinned release.
 */
export function docsRsUrl(context: SwiftideContext, target: string): string {
	const crate = target.split(/[/#]/, 1)[0]!;
	if (!context.crates.includes(crate)) {
		throw new SwiftideContextError(
			`Unknown crate "${crate}" in docsrs:${target}. Known crates: ${context.crates.join(', ')}.`,
		);
	}
	const rest = target.slice(crate.length);
	const tail = rest === '' ? '/' : rest.startsWith('#') ? `/${rest}` : rest;
	return `https://docs.rs/${crate.replaceAll('_', '-')}/${context.version}/${crate}${tail}`;
}

/** GitHub permalink to a file (and optional line range) in the pinned swiftide commit. */
export function swiftidePermalink(
	context: SwiftideContext,
	relPath: string,
	lines?: { start: number; end: number },
): string {
	const anchor = lines ? `#L${lines.start}-L${lines.end}` : '';
	return `${context.ghBlob}${relPath}${anchor}`;
}
