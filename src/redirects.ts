/**
 * Every URL of the previous site and its new home. `scripts/legacy-urls.txt` lists the same
 * URLs; `pnpm verify:dist` checks each one still resolves or redirects.
 *
 * `/indexing/chunking/`, `/querying/transforming-queries/` and `/agents/overview/` kept their
 * slugs and need no redirect.
 */
export const redirects: Record<string, string> = {
	'/what-is-swiftide': '/start/what-is-swiftide/',
	'/getting-started/installation': '/start/installation/',
	'/getting-started/architecture-and-design': '/start/what-is-swiftide/',
	'/getting-started/features': '/integrations/overview/',
	'/getting-started/changelog': '/reference/changelog/',
	'/core-concepts/indexing-pipeline': '/indexing/overview/',
	'/core-concepts/query-pipeline': '/querying/overview/',
	'/core-concepts/prompting-embedding': '/models/overview/',
	'/indexing/loading-data': '/indexing/loading/',
	'/indexing/transforming-and-enriching': '/indexing/transformers-and-metadata/',
	'/indexing/caching-and-filtering': '/indexing/caching/',
	'/indexing/storing-results': '/indexing/storage/',
	'/querying/search-strategies': '/querying/retrieval-and-search-strategies/',
	'/querying/retrieving-data': '/querying/retrieval-and-search-strategies/',
	'/querying/transforming-response': '/querying/reranking-and-response-transformers/',
	'/querying/generating-answer': '/querying/answering/',
	'/agents/hello-agents': '/start/your-first-agent/',
	'/agents/creating-tools': '/agents/tools/',
	'/agents/lifecycle-hooks': '/agents/hooks/',
	'/in-depth/customizing-prompts': '/models/prompts-and-templates/',
	'/in-depth/splitting-and-merging-streams': '/indexing/streams-and-concurrency/',
	'/in-depth/streaming-and-concurrency': '/indexing/streams-and-concurrency/',
	'/tutorials/indexing-and-querying-code': '/showcases/ask-your-codebase/',
	'/troubleshooting': '/reference/troubleshooting-and-faq/',
};
