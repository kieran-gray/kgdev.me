import { siteConfig } from '@/config/site';

export interface PersonSchema {
	'@context': 'https://schema.org';
	'@type': 'Person';
	name: string;
	url: string;
	sameAs: string[];
	jobTitle: string;
	description: string;
}

export interface BlogPostingSchema {
	'@context': 'https://schema.org';
	'@type': 'BlogPosting';
	headline: string;
	description: string;
	author: { '@type': 'Person'; name: string; url: string };
	publisher: { '@type': 'Person'; name: string; url: string };
	datePublished: string;
	url: string;
	image: string;
	keywords: string;
}

export function buildPersonSchema(): PersonSchema {
	return {
		'@context': 'https://schema.org',
		'@type': 'Person',
		name: siteConfig.author.name,
		url: siteConfig.url,
		sameAs: siteConfig.social
			.filter(
				(s) =>
					s.kind === 'github' ||
					s.kind === 'linkedin' ||
					s.kind === 'twitter' ||
					s.kind === 'mastodon'
			)
			.map((s) => s.href),
		jobTitle: siteConfig.author.jobTitle,
		description: siteConfig.author.bio
	};
}

export function buildBlogPostingSchema(args: {
	title: string;
	description: string;
	pubDate: Date | string;
	author: string;
	tags: string[];
	url: string;
	image: string;
}): BlogPostingSchema {
	const personUrl = siteConfig.url;
	return {
		'@context': 'https://schema.org',
		'@type': 'BlogPosting',
		headline: args.title,
		description: args.description,
		author: { '@type': 'Person', name: args.author, url: personUrl },
		publisher: { '@type': 'Person', name: args.author, url: personUrl },
		datePublished: new Date(args.pubDate).toISOString(),
		url: args.url,
		image: args.image,
		keywords: (args.tags ?? []).join(', ')
	};
}
