import type { BlogFeature } from './_types';
import type { Features } from '@/config/features';
import { createContactFeature } from './contact/feature';
import { createOgFeature } from './og/feature';
import { createRssFeature } from './rss/feature';
import { createSearchFeature } from './search/feature';
import { createViewCounterFeature } from './view-counter/feature';
import { createMermaidFeature } from './mermaid/feature';
import { createProjectsFeature } from './projects/feature';
import { createBooksFeature } from './books/feature';

export function getBlogFeatures(flags: Features): BlogFeature[] {
	return [
		createContactFeature(flags.contact.enabled),
		createOgFeature(flags.og.enabled),
		createRssFeature(flags.rss.enabled),
		createSearchFeature(flags.search.enabled),
		createViewCounterFeature(flags.viewCounter.enabled),
		createMermaidFeature(flags.mermaid.enabled),
		createProjectsFeature(flags.projects.enabled),
		createBooksFeature(flags.books.enabled)
	];
}
