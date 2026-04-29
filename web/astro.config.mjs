import { defineConfig } from 'astro/config';
import { remarkReadingTime } from './src/lib/markdown/readingTime';
import rehypePrettyCode from 'rehype-pretty-code';
import tailwindcss from '@tailwindcss/vite';
import react from '@astrojs/react';
import sitemap from '@astrojs/sitemap';
import mermaid from 'astro-mermaid';
import cloudflare from '@astrojs/cloudflare';
import { features } from './src/config/features.mjs';
import { readdirSync, readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const SITE_URL = process.env.PUBLIC_SITE_URL ?? 'https://kgdev.me';

function buildPostDateMap() {
	const postsDir = join(__dirname, 'src/content/posts');
	const map = {};
	try {
		const files = readdirSync(postsDir).filter((f) => f.endsWith('.md'));
		for (const file of files) {
			const content = readFileSync(join(postsDir, file), 'utf-8');
			const match = content.match(/pubDate:\s*(\d{4}-\d{2}-\d{2})/);
			if (match) {
				const slug = file.replace('.md', '');
				const url = new URL(`/posts/${slug}/`, SITE_URL).toString();
				map[url] = new Date(match[1]);
			}
		}
	} catch {}
	return map;
}

const postDateMap = buildPostDateMap();
const options = {
	onVisitLine(node) {
		if (node.children.length === 0) {
			node.children = [
				{
					type: 'text',
					value: ' '
				}
			];
		}
	},
	onVisitHighlightedLine(node) {
		node.properties.className = ['highlighted'];
	}
};

function optionalRoutes() {
	return {
		name: 'optional-routes',
		hooks: {
			'astro:config:setup': ({ injectRoute }) => {
				if (features.contact.enabled) {
					injectRoute({
						pattern: '/contact',
						entrypoint: './src/optional-routes/contact/index.astro',
						prerender: true
					});
				}

				if (features.og.enabled) {
					injectRoute({
						pattern: '/og/default.png',
						entrypoint: './src/optional-routes/og/default.png.ts',
						prerender: true
					});
					injectRoute({
						pattern: '/og/[slug].png',
						entrypoint: './src/optional-routes/og/[slug].png.ts',
						prerender: true
					});
				}

				if (features.rss.enabled) {
					injectRoute({
						pattern: '/rss.xml',
						entrypoint: './src/optional-routes/rss.xml.ts',
						prerender: true
					});
				}
			}
		}
	};
}

export default defineConfig({
	site: SITE_URL,

	markdown: {
		syntaxHighlight: false,
		rehypePlugins: [[rehypePrettyCode, options]],
		remarkPlugins: [remarkReadingTime]
	},

	integrations: [
		optionalRoutes(),
		features.mermaid.enabled &&
			mermaid({
				theme: 'neutral',
				autoTheme: true,
				enableLog: false,
				mermaidConfig: {
					flowchart: { curve: 'linear' }
				}
			}),
		react(),
		sitemap({
			filter(page) {
				const pathname = typeof page === 'string' ? new URL(page).pathname : '';
				if (!features.contact.enabled && pathname === '/contact') return false;
				if (!features.rss.enabled && pathname === '/rss.xml') return false;
				if (!features.og.enabled && pathname.startsWith('/og/')) return false;
				return true;
			},
			serialize(item) {
				return {
					...item,
					lastmod: postDateMap[item.url] ?? new Date()
				};
			}
		})
	].filter(Boolean),

	output: 'static',

	vite: {
		plugins: [tailwindcss()],
		ssr: {
			external: ['@resvg/resvg-js', 'satori', 'node:fs', 'node:path']
		}
	},

	adapter: cloudflare()
});
