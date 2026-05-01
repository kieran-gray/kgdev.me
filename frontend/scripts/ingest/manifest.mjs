import { readFileSync, writeFileSync, existsSync } from 'node:fs';

export function readManifest(path) {
	if (!existsSync(path)) return { version: 1, posts: {} };
	const raw = JSON.parse(readFileSync(path, 'utf8'));
	if (!raw || typeof raw !== 'object') return { version: 1, posts: {} };
	return { version: raw.version ?? 1, posts: raw.posts ?? {} };
}

export function writeManifest(path, manifest) {
	const ordered = {
		version: manifest.version ?? 1,
		posts: Object.fromEntries(
			Object.keys(manifest.posts).sort().map((k) => [k, manifest.posts[k]]),
		),
	};
	writeFileSync(path, JSON.stringify(ordered, null, 2) + '\n');
}

export function recordPost(manifest, slug, entry) {
	manifest.posts[slug] = {
		post_version: entry.post_version,
		chunk_count: entry.chunk_count,
		ingested_at: entry.ingested_at,
	};
	return manifest;
}

export function previousEntry(manifest, slug) {
	return manifest.posts[slug] ?? null;
}
