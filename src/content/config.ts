import { defineCollection, z } from 'astro:content';

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
	type: 'content',
	schema: z.object({
		name: z.string(),
		summary: z.string(),
		website: z.string().url().optional(),
		status: z.enum(['active', 'paused', 'archived']).default('active'),
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
			.optional()
	})
});

export const collections = { projects };
