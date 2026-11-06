/**
 * The release-notes agent's recorded run, as shown on the home page hero.
 *
 * The lines are cut from the ```text block in the showcase's OUTPUT.md (a real run, committed in
 * swiftide), so the page never shows output that was not printed. The selection is by exact line
 * and in order; if the recording changes and a line is gone, the build fails here.
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';

const OUTPUT_FILE = 'vendor/swiftide/examples/release-notes-agent/OUTPUT.md';

/** The command that produced the run, as given in OUTPUT.md. */
export const RELEASE_RUN_COMMAND = 'release-notes-agent --since v0.33.0 --version 0.34.0';

/** The lines shown, in the order they were printed. */
const SHOWN = [
	'→ git_log_since v0.33.0',
	'git_log_since ✓ 346 lines',
	'→ git_diff swiftide-integrations/Cargo.toml 3b1ad22e^..3b1ad22e',
	'→ git_diff …/src/chat_completion/chat_completion_response.rs 90bcadc7^..90bcadc7',
	'→ git_diff swiftide-agents-contrib/Cargo.toml 71fa4587^..71fa4587',
	'git_diff ✓ 15 lines',
	'git_diff ✓ 72 lines',
	'git_diff ✓ 75 lines',
	'→ write_changelog <32 lines>',
	'write_changelog ⏸ waiting for approval',
	'Write these release notes? [y/N] y',
	'write_changelog ✓ wrote RELEASE_NOTES.md',
	'→ stop (version 0.34.0: highlights 2, breaking 1, fixes 1)',
	'stop ■ done',
] as const;

/** The first fenced text block of the recording, one entry per printed line, indentation removed. */
function recordedLines(projectRoot: string): string[] {
	const source = readFileSync(path.join(projectRoot, OUTPUT_FILE), 'utf8');
	const block = /^(`{3,})text\n([\s\S]*?)^\1$/m.exec(source);
	if (!block) throw new Error(`${OUTPUT_FILE}: no \`\`\`text block with the recorded run`);
	return block[2]!.split('\n').map((line) => line.trim());
}

/** The shown lines, checked against the recording: each must appear, after the one before it. */
export function loadReleaseRun(projectRoot: string): readonly string[] {
	const recorded = recordedLines(projectRoot);
	let from = 0;
	for (const line of SHOWN) {
		const at = recorded.indexOf(line, from);
		if (at < 0) throw new Error(`${OUTPUT_FILE}: the hero shows "${line}", which the recorded run no longer prints in this order`);
		from = at + 1;
	}
	return SHOWN;
}
