import { getCollection, type CollectionEntry } from 'astro:content';
import { features } from '@/config/features';

export type ProjectEntry = CollectionEntry<'projects'>;

function sortProjects(projects: ProjectEntry[]): ProjectEntry[] {
	return [...projects].sort(
		(a, b) => Number(b.data.pinned === true) - Number(a.data.pinned === true)
	);
}

export function getProjectTags(project: ProjectEntry): string[] {
	return [...(project.data.tags || []), ...(project.data.tech?.languages || [])];
}

export function getProjectSlug(project: ProjectEntry): string {
	return project.id.replace(/\.md$/, '');
}

export async function getAllProjects(): Promise<ProjectEntry[]> {
	if (!features.projects.enabled) return [];
	const projects = await getCollection('projects');
	return sortProjects(projects);
}

export async function getProjectsByTag(tag: string): Promise<ProjectEntry[]> {
	const projects = await getAllProjects();
	return projects.filter((project) => getProjectTags(project).includes(tag));
}

export async function getHomepageProjects(): Promise<{
	projects: ProjectEntry[];
	title: string;
}> {
	if (!features.projects.enabled) {
		return { projects: [], title: 'Projects' };
	}
	const projects = await getAllProjects();
	const pinnedProjects = projects.filter((project) => project.data.pinned === true);
	return {
		projects: (pinnedProjects.length ? pinnedProjects : projects).slice(0, 2),
		title: pinnedProjects.length ? 'Pinned Projects' : 'Latest Projects'
	};
}

export async function getProjectBySlug(slug: string): Promise<ProjectEntry | undefined> {
	const projects = await getAllProjects();
	return projects.find((project) => getProjectSlug(project) === slug);
}
