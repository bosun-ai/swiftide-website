/**
 * Version context of the pinned swiftide checkout.
 *
 * The superproject's gitlink for `vendor/swiftide` is the single source of truth for which
 * swiftide the site documents. Everything that mentions a version, a docs.rs URL or a GitHub
 * permalink derives it from here. A checkout that does not match the gitlink is an error: the site
 * must never be built against an unknown swiftide.
 *
 * The release version comes from the `swiftide` crate's manifest at the pinned commit. Swiftide
 * releases with release-plz: the release PR bumps the manifests and the `v<version>` tag lands on
 * that same commit later, and between releases the manifest still names the last release. So the
 * manifest is right at any pin, including a release commit whose tag is not pushed yet.
 */
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';

export const SWIFTIDE_REPO = 'https://github.com/bosun-ai/swiftide';
export const VENDOR_DIR = 'vendor/swiftide';

export interface SwiftideContext {
	/** Absolute path to the swiftide checkout. */
	readonly dir: string;
	/** Full commit SHA recorded by the gitlink. */
	readonly sha: string;
	readonly shortSha: string;
	/** Release tag of `version`, e.g. `v0.34.0`. It may not be pushed yet for a fresh release. */
	readonly tag: string;
	/** Version of the `swiftide` crate at `sha`, e.g. `0.34.0`: the release the pin belongs to. */
	readonly version: string;
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

const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?$/;
const TABLE_HEADER = /^\s*\[([^[\]]+)\]\s*(?:#.*)?$/;

/** Raw right-hand side of `key = …` in `[table]`, or `undefined`. Only single-line values. */
function tableValue(toml: string, table: string, key: string): string | undefined {
	let current: string | undefined;
	for (const line of toml.split(/\r?\n/)) {
		const header = TABLE_HEADER.exec(line);
		if (header) {
			current = header[1]!.trim();
			continue;
		}
		if (current !== table) continue;
		const entry = /^\s*([A-Za-z0-9_.-]+)\s*=\s*(.*?)\s*$/.exec(line);
		if (entry?.[1] === key) return entry[2];
	}
	return undefined;
}

function quotedVersion(value: string | undefined): string | undefined {
	const version = value === undefined ? undefined : /^"([^"]*)"\s*(?:#.*)?$/.exec(value)?.[1];
	return version !== undefined && SEMVER.test(version) ? version : undefined;
}

/**
 * The `[package]` version of a crate manifest, following `version.workspace = true` into the
 * workspace manifest's `[workspace.package]`. `undefined` when no valid version is declared.
 */
export function parsePackageVersion(crateToml: string, workspaceToml: string): string | undefined {
	const inherits =
		/^true\s*(?:#.*)?$/.test(tableValue(crateToml, 'package', 'version.workspace') ?? '') ||
		/^\{\s*workspace\s*=\s*true\s*\}/.test(tableValue(crateToml, 'package', 'version') ?? '');
	return inherits
		? quotedVersion(tableValue(workspaceToml, 'workspace.package', 'version'))
		: quotedVersion(tableValue(crateToml, 'package', 'version'));
}

function swiftideVersion(dir: string): string {
	const crateManifest = path.join(dir, 'swiftide', 'Cargo.toml');
	const version = existsSync(crateManifest)
		? parsePackageVersion(readFileSync(crateManifest, 'utf8'), readFileSync(path.join(dir, 'Cargo.toml'), 'utf8'))
		: undefined;
	if (!version) {
		throw new SwiftideContextError(
			`Cannot read the swiftide version from ${VENDOR_DIR}/swiftide/Cargo.toml ` +
				'(`[package] version`, or `[workspace.package] version` of the workspace Cargo.toml).',
		);
	}
	return version;
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
	const version = swiftideVersion(dir);
	return {
		dir,
		sha,
		shortSha: sha.slice(0, 9),
		tag: `v${version}`,
		version,
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
