import { defineCollection, z } from 'astro:content';
import { file, glob } from 'astro/loaders';

const hostingSchema = z.object({
	provider: z.string(),
	service: z.string().optional(),
	url: z.string().url().optional()
});

const componentSchema = z.object({
	name: z.string(),
	type: z.enum(['frontend', 'backend', 'infra', 'worker', 'db', 'mobile']),
	language: z.string(),
	framework: z.string().optional(),
	repo: z.string().optional(),
	github: z.string().url().optional(),
	packageManager: z.string().optional(),
	hosting: hostingSchema.optional(),
	notes: z.array(z.string()).optional()
});

const repoSchema = z.object({
	name: z.string(),
	url: z.string().url(),
	role: z.string(),
	private: z.boolean().optional()
});

const projects = defineCollection({
	loader: glob({
		base: new URL('../features/projects/content', import.meta.url),
		pattern: '**/*.md',
		generateId: ({ entry }) => entry.replace(/\.md$/, ''),
		_legacy: true
	}),
	schema: z.object({
		name: z.string(),
		summary: z.string(),
		website: z.string().url().optional(),
		status: z.enum(['active', 'paused', 'archived']).default('active'),
		pinned: z.boolean().default(false),
		tags: z.array(z.string()).default([]),
		tech: z
			.object({
				languages: z.array(z.string()).default([]),
				frameworks: z.array(z.string()).default([])
			})
			.default({ languages: [], frameworks: [] }),
		hosting: z.array(hostingSchema).default([]),
		repos: z.array(repoSchema).default([]),
		components: z.array(componentSchema).default([]),
		images: z
			.object({
				logo: z.string().optional(),
				architecture: z.string().optional()
			})
			.optional(),
		insights: z
			.object({
				performance: z
					.object({
						image: z.string().optional(),
						notes: z.array(z.string()).default([])
					})
					.default({ notes: [] }),
				security: z
					.object({
						image: z.string().optional(),
						notes: z.array(z.string()).default([])
					})
					.default({ notes: [] })
			})
			.optional()
	})
});

const books = defineCollection({
	loader: file('src/features/books/content/books.json'),
	schema: z.object({
		id: z.string(),
		title: z.string(),
		author: z.string(),
		rating: z.number().int().min(0).max(5),
		dateRead: z.string().nullable(),
		shelf: z.string(),
		readCount: z.number().int().min(0)
	})
});

const posts = defineCollection({
	type: 'content',
	schema: z.object({
		title: z.string(),
		description: z.string(),
		pubDate: z.coerce.date(),
		author: z.string(),
		excerpt: z.string(),
		tags: z.array(z.string()).default([]),
		isPinned: z.boolean().default(false),
		qaPlaceholder: z.string().optional(),
		glossaryTerms: z.array(z.string()).default([]),
		image: z.object({ src: z.string().optional(), alt: z.string().optional() }).default({})
	})
});

const glossary = defineCollection({
	type: 'content',
	schema: z.object({
		term: z.string(),
		sources: z
			.array(
				z.object({
					title: z.string(),
					url: z.string().url()
				})
			)
			.default([])
	})
});

const pages = defineCollection({
	type: 'content',
	schema: z.object({
		headline: z.string(),
		currentRole: z
			.object({
				title: z.string(),
				org: z.string().optional(),
				orgUrl: z.string().url().optional(),
				summary: z.string().optional()
			})
			.optional()
	})
});

export const collections = { projects, posts, pages, books, glossary };
