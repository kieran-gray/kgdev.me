import { getAllPosts, type PostEntry } from './posts';
import { getAllProjects, getProjectTags, type ProjectEntry } from './projects';
import { features } from '@/config/features';

export interface TagCounts {
	posts: number;
	projects: number;
	total: number;
}

export interface TagIndexEntry {
	tag: string;
	counts: TagCounts;
}

export interface TaggedContent {
	tag: string;
	posts: PostEntry[];
	projects: ProjectEntry[];
}

export async function getTagIndex(): Promise<TagIndexEntry[]> {
	const [posts, projects] = await Promise.all([getAllPosts(), getAllProjects()]);
	const counts = new Map<string, TagCounts>();

	for (const post of posts) {
		for (const tag of post.data.tags || []) {
			const existing = counts.get(tag) ?? { posts: 0, projects: 0, total: 0 };
			existing.posts += 1;
			existing.total += 1;
			counts.set(tag, existing);
		}
	}

	for (const project of projects) {
		for (const tag of getProjectTags(project)) {
			const existing = counts.get(tag) ?? { posts: 0, projects: 0, total: 0 };
			existing.projects += 1;
			existing.total += 1;
			counts.set(tag, existing);
		}
	}

	return [...counts.entries()]
		.map(([tag, tagCounts]) => ({ tag, counts: tagCounts }))
		.sort((a, b) => b.counts.total - a.counts.total || a.tag.localeCompare(b.tag));
}

export async function getTaggedContent(tag: string): Promise<TaggedContent> {
	const [posts, projects] = await Promise.all([getAllPosts(), getAllProjects()]);
	return {
		tag,
		posts: posts.filter((post) => (post.data.tags || []).includes(tag)),
		projects: projects.filter((project) => getProjectTags(project).includes(tag))
	};
}

export function getTagBrowseDescription(): string {
	return features.projects.enabled
		? 'Browse all posts and projects by tag.'
		: 'Browse all posts by tag.';
}

export function getTaggedContentDescription(tag: string): string {
	return features.projects.enabled
		? `All posts and projects tagged "${tag}".`
		: `All posts tagged "${tag}".`;
}
