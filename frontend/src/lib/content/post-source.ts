import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { getCollection, type CollectionEntry } from 'astro:content';
import { siteConfig } from '@/config/site';
import { markdownToPlainText } from '@/lib/markdown/plainText';

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
}

const postsDir = resolve(process.cwd(), 'src/content/posts');

export function stripFrontmatter(source: string): string {
	if (!source.startsWith('---\n')) return source;
	const end = source.indexOf('\n---\n', 4);
	if (end === -1) return source;
	return source.slice(end + 5);
}

function sha256Hex(value: string): string {
	return createHash('sha256').update(value).digest('hex');
}

function toAbsoluteUrl(pathname: string): string {
	return new URL(pathname, siteConfig.url).toString();
}

export function readPostSource(slug: string): string {
	return readFileSync(resolve(postsDir, `${slug}.md`), 'utf8');
}

export function buildPostSourcePayload(entry: CollectionEntry<'posts'>): PostSourcePayload {
	const sourceMarkdown = readPostSource(entry.slug);
	const markdownBody = stripFrontmatter(sourceMarkdown).trim();
	const canonicalPath = `/posts/${entry.slug}/`;
	const rawMarkdownPath = `/posts/${entry.slug}.md`;
	const jsonPath = `/api/posts/${entry.slug}.json`;

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
		contentHash: sha256Hex(sourceMarkdown),
		sourceMarkdown,
		markdownBody,
		plainText: markdownToPlainText(markdownBody)
	};
}

export async function getAllPostSourcePayloads(): Promise<PostSourcePayload[]> {
	const entries: CollectionEntry<'posts'>[] = await getCollection('posts');
	return entries
		.sort((a, b) => b.data.pubDate.getTime() - a.data.pubDate.getTime())
		.map(buildPostSourcePayload);
}
