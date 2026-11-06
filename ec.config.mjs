// @ts-check
// Expressive Code configuration: the swiftide themes (palettes mirror src/styles/tokens.css) and
// style overrides that make code frames flat, square and mono. The collapsible-sections plugin is
// needed because `::snippet{collapse="…"}` emits `collapse={…}`.
import { readFileSync } from 'node:fs';
import { pluginCollapsibleSections } from '@expressive-code/plugin-collapsible-sections';
import { defineEcConfig, ExpressiveCodeTheme } from '@astrojs/starlight/expressive-code';

/** @param {string} name */
const theme = (name) =>
	ExpressiveCodeTheme.fromJSONString(readFileSync(new URL(`./src/data/ec-themes/${name}.json`, import.meta.url), 'utf8'));

export default defineEcConfig({
	themes: [theme('swiftide-dark'), theme('swiftide-light')],
	useStarlightUiThemeColors: false,
	plugins: [pluginCollapsibleSections()],
	defaultProps: {
		collapseStyle: 'collapsible-auto',
		// Shell snippets are commands to run: always a terminal frame.
		overridesByLang: {
			'sh,bash,shell,zsh,console': { frame: 'terminal' },
		},
	},
	styleOverrides: {
		borderRadius: '0',
		borderWidth: '1px',
		borderColor: 'var(--sw-line)',
		codeFontFamily: 'var(--sw-font-mono)',
		codeFontSize: 'var(--sw-text-code)',
		codeLineHeight: '1.7',
		codePaddingBlock: '0.875rem',
		codePaddingInline: '1.125rem',
		codeBackground: 'var(--sw-code-bg)',
		codeForeground: 'var(--sw-code-ink)',
		uiFontFamily: 'var(--sw-font-mono)',
		uiFontSize: '0.75rem',
		focusBorder: 'var(--sw-accent)',
		scrollbarThumbColor: 'var(--sw-line-strong)',
		frames: {
			shadowColor: 'transparent',
			frameBoxShadowCssValue: 'none',
			editorTabBarBackground: 'var(--sw-code-titlebar)',
			editorTabBarBorderColor: 'var(--sw-line)',
			editorTabBarBorderBottomColor: 'var(--sw-line)',
			editorActiveTabBackground: 'var(--sw-code-bg)',
			editorActiveTabForeground: 'var(--sw-ink)',
			editorActiveTabBorderColor: 'var(--sw-line)',
			editorActiveTabIndicatorHeight: '1px',
			editorActiveTabIndicatorTopColor: 'var(--sw-accent)',
			editorActiveTabIndicatorBottomColor: 'transparent',
			editorTabBorderRadius: '0',
			editorBackground: 'var(--sw-code-bg)',
			terminalBackground: 'var(--sw-code-bg)',
			terminalTitlebarBackground: 'var(--sw-code-titlebar)',
			terminalTitlebarForeground: 'var(--sw-muted)',
			terminalTitlebarBorderBottomColor: 'var(--sw-line)',
			terminalTitlebarDotsForeground: 'var(--sw-line-strong)',
			terminalTitlebarDotsOpacity: '1',
			inlineButtonBackground: 'var(--sw-ink)',
			inlineButtonForeground: 'var(--sw-ink)',
			inlineButtonBorder: 'var(--sw-line-strong)',
			inlineButtonBorderOpacity: '1',
			tooltipSuccessBackground: 'var(--sw-ink-strong)',
			tooltipSuccessForeground: 'var(--sw-bg)',
		},
		collapsibleSections: {
			closedBackgroundColor: 'var(--sw-panel-strong)',
			closedBorderColor: 'var(--sw-line)',
			closedTextColor: 'var(--sw-muted)',
			closedFontFamily: 'var(--sw-font-mono)',
			closedFontSize: '0.75rem',
			openBackgroundColorCollapsible: 'transparent',
			openBorderColor: 'var(--sw-line)',
		},
		textMarkers: {
			markBackground: 'var(--sw-accent-wash)',
			markBorderColor: 'var(--sw-accent)',
		},
	},
});
