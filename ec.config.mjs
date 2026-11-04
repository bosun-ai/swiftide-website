// @ts-check
// Expressive Code configuration. Worker B adds the swiftide themes and style overrides; the
// collapsible-sections plugin is needed now because `::snippet{collapse="…"}` emits `collapse={…}`.
import { pluginCollapsibleSections } from '@expressive-code/plugin-collapsible-sections';
import { defineEcConfig } from '@astrojs/starlight/expressive-code';

export default defineEcConfig({
	plugins: [pluginCollapsibleSections()],
	defaultProps: { collapseStyle: 'collapsible-auto' },
});
