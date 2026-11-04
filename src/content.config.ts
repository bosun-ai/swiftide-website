import { defineCollection } from 'astro:content';
import { docsSchema } from '@astrojs/starlight/schema';
import { z } from 'astro/zod';
import { swiftideDocsLoader } from './lib/content-loader.ts';

export const collections = {
	docs: defineCollection({
		loader: swiftideDocsLoader(),
		schema: docsSchema({
			extend: z.object({
				/** Version badges shown next to the page title. */
				swiftide: z
					.object({
						/** First swiftide release with this feature, e.g. `0.32`. */
						since: z.string().optional(),
						/** On master, not in a release yet. */
						unreleased: z.boolean().default(false),
					})
					.optional(),
				/** Inline code the identifier lint should not check on this page. */
				identLint: z.object({ ignore: z.array(z.string()).default([]) }).optional(),
			}),
		}),
	}),
};
