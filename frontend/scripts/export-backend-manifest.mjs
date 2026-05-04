import { readdirSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const frontendRoot = resolve(scriptDir, '..');
const repoRoot = resolve(frontendRoot, '..');
const postsDir = resolve(frontendRoot, 'src/content/posts');
const outputPath = resolve(repoRoot, 'backend/generated/blog-manifest.json');
const slugPattern = /^(?!-)(?!.*--)[a-z0-9]+(?:-[a-z0-9]+)*$/;

const posts = readdirSync(postsDir, { withFileTypes: true })
	.filter((entry) => entry.isFile() && entry.name.endsWith('.md'))
	.map((entry) => entry.name.replace(/\.md$/, ''))
	.sort();

if (posts.length === 0) {
	throw new Error(`No posts found in ${postsDir}`);
}

for (const slug of posts) {
	if (!slugPattern.test(slug)) {
		throw new Error(`Post filename does not produce a valid backend slug: ${slug}`);
	}
}

mkdirSync(dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${JSON.stringify({ posts }, null, 2)}\n`);

console.log(`Wrote backend blog manifest with ${posts.length} posts: ${outputPath}`);
