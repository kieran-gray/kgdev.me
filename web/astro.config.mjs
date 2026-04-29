import { defineConfig } from 'astro/config';
import { remarkReadingTime } from './src/utils/readingTime';
import rehypePrettyCode from 'rehype-pretty-code';
import tailwindcss from '@tailwindcss/vite';
import react from '@astrojs/react';
import sitemap from '@astrojs/sitemap';
import mermaid from 'astro-mermaid';
import cloudflare from "@astrojs/cloudflare";
import { readdirSync, readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

function buildPostDateMap() {
	const postsDir = join(__dirname, 'src/pages/posts');
	const map = {};
	try {
		const files = readdirSync(postsDir).filter(f => f.endsWith('.md'));
		for (const file of files) {
			const content = readFileSync(join(postsDir, file), 'utf-8');
			const match = content.match(/pubDate:\s*(\d{4}-\d{2}-\d{2})/);
			if (match) {
				const slug = file.replace('.md', '');
				map[`https://kgdev.me/posts/${slug}/`] = new Date(match[1]);
			}
		}
	} catch {}
	return map;
}

const postDateMap = buildPostDateMap();
const options = {
	// Specify the theme to use or a custom theme json, in our case
	// it will be a moonlight-II theme from
	// https://github.com/atomiks/moonlight-vscode-theme/blob/master/src/moonlight-ii.json
	// Callbacks to customize the output of the nodes
	//theme: json,
	onVisitLine(node) {
		// Prevent lines from collapsing in `display: grid` mode, and
		// allow empty lines to be copy/pasted
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
		// Adding a class to the highlighted line
		node.properties.className = ['highlighted'];
	}
};

// https://astro.build/config
export default defineConfig({
    site: 'https://kgdev.me/',

    markdown: {
		syntaxHighlight: false,
		// Disable syntax built-in syntax hightlighting from astro
		rehypePlugins: [[rehypePrettyCode, options]],
		remarkPlugins: [remarkReadingTime]
	},

    integrations: [
		mermaid({
			theme: 'neutral',
			autoTheme: true,
			enableLog: false,
			mermaidConfig: {
				flowchart: { curve: 'linear' },
			},
		}),
		react(),
		sitemap({
			serialize(item) {
				return {
					...item,
					lastmod: postDateMap[item.url] ?? new Date(),
				};
			},
		}),
	],

    output: 'static',

    vite: {
		plugins: [tailwindcss()],
		ssr: {
			external: ['@resvg/resvg-js', 'satori', 'node:fs', 'node:path']
		}
	},

    adapter: cloudflare()
});