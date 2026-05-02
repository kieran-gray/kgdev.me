import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { getCollection, type CollectionEntry } from 'astro:content';
import { siteConfig } from '@/config/site';
import { markdownToPlainText } from '@/lib/markdown/plainText';

export interface GlossaryTermSource {
	title: string;
	url: string;
}

export interface GlossaryTerm {
	slug: string;
	term: string;
	definition: string;
	sources: GlossaryTermSource[];
}

export interface PostSourcePayload {
	slug: string;
	title: string;
	description: string;
	excerpt: string;
	author: string;
	publishedAt: string;
	tags: string[];
	canonicalUrl: string;
	rawMarkdownUrl: string;
	jsonUrl: string;
	contentHash: string;
	sourceMarkdown: string;
	markdownBody: string;
	plainText: string;
	glossaryTerms: GlossaryTerm[];
}

const postsDir = resolve(process.cwd(), 'src/content/posts');
const glossaryDir = resolve(process.cwd(), 'src/content/glossary');

export function stripFrontmatter(source: string): string {
	if (!source.startsWith('---\n')) return source;
	const end = source.indexOf('\n---\n', 4);
	if (end === -1) return source;
	return source.slice(end + 5);
}

function toAbsoluteUrl(pathname: string): string {
	return new URL(pathname, siteConfig.url).toString();
}

export function readPostSource(slug: string): string {
	return readFileSync(resolve(postsDir, `${slug}.md`), 'utf8');
}

export function readGlossarySource(slug: string): string {
	return readFileSync(resolve(glossaryDir, `${slug}.md`), 'utf8');
}

export async function buildPostSourcePayload(
	entry: CollectionEntry<'posts'>
): Promise<PostSourcePayload> {
	const sourceMarkdown = readPostSource(entry.slug);
	const markdownBody = stripFrontmatter(sourceMarkdown).trim();
	const canonicalPath = `/posts/${entry.slug}/`;
	const rawMarkdownPath = `/posts/${entry.slug}.md`;
	const jsonPath = `/api/posts/${entry.slug}.json`;

	const glossaryEntries = await getCollection('glossary');
	const glossaryTerms: GlossaryTerm[] = (entry.data.glossaryTerms ?? [])
		.map((slug: string) => {
			const glossaryEntry = glossaryEntries.find(
				(e: CollectionEntry<'glossary'>) => e.slug === slug
			);
			if (!glossaryEntry) return null;

			const source = readGlossarySource(slug);
			const definition = stripFrontmatter(source).trim();

			return {
				slug,
				term: glossaryEntry.data.term,
				definition,
				sources: glossaryEntry.data.sources
			};
		})
		.filter((t: GlossaryTerm | null): t is GlossaryTerm => t !== null);

	const glossaryForHashing = glossaryTerms.map((t) => ({
		term: t.term,
		definition: t.definition,
		sources: t.sources.map((s) => ({
			title: s.title,
			url: s.url
		}))
	}));
	const glossaryJson = JSON.stringify(glossaryForHashing);

	const contentHash = createHash('sha256')
		.update(sourceMarkdown)
		.update(glossaryJson)
		.digest('hex');

	return {
		slug: entry.slug,
		title: entry.data.title,
		description: entry.data.description,
		excerpt: entry.data.excerpt,
		author: entry.data.author,
		publishedAt: entry.data.pubDate.toISOString(),
		tags: entry.data.tags ?? [],
		canonicalUrl: toAbsoluteUrl(canonicalPath),
		rawMarkdownUrl: toAbsoluteUrl(rawMarkdownPath),
		jsonUrl: toAbsoluteUrl(jsonPath),
		contentHash,
		sourceMarkdown,
		markdownBody,
		plainText: markdownToPlainText(markdownBody),
		glossaryTerms
	};
}

export async function getAllPostSourcePayloads(): Promise<PostSourcePayload[]> {
	const entries: CollectionEntry<'posts'>[] = await getCollection('posts');
	const payloads = await Promise.all(
		entries
			.sort((a, b) => b.data.pubDate.getTime() - a.data.pubDate.getTime())
			.map((entry) => buildPostSourcePayload(entry))
	);
	return payloads;
}

export async function getAllGlossaryTerms(): Promise<GlossaryTerm[]> {
	const entries = await getCollection('glossary');
	return entries.map((entry: CollectionEntry<'glossary'>) => {
		const source = readGlossarySource(entry.slug);
		const definition = stripFrontmatter(source).trim();
		return {
			slug: entry.slug,
			term: entry.data.term,
			definition,
			sources: entry.data.sources
		};
	});
}
