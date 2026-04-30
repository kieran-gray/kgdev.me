import { defineConfig } from 'astro/config';
import { remarkReadingTime } from './src/lib/markdown/readingTime';
import rehypePrettyCode from 'rehype-pretty-code';
import tailwindcss from '@tailwindcss/vite';
import sitemap from '@astrojs/sitemap';
import cloudflare from '@astrojs/cloudflare';
import type { AstroIntegration } from 'astro';
import { siteConfig } from './src/config/site';
import { getBlogFeatures } from './src/features';
// @ts-ignore Shared plain-JS env parser used by both Node scripts and Astro config.
import { getFeatureFlags } from './src/config/feature-flags.mjs';
// @ts-ignore Shared plain-JS env loader used by both Node scripts and Astro config.
import { getWranglerPublicVars } from './src/config/wrangler-env.mjs';

const publicEnv = getWranglerPublicVars({ env: process.env });
const featureFlags = getFeatureFlags(publicEnv);
const blogFeatures = getBlogFeatures(featureFlags);
const featureIntegrations: AstroIntegration[] = blogFeatures
	.map((f) => f.integration)
	.filter((i): i is AstroIntegration => Boolean(i));
const sitemapExclusions = blogFeatures
	.filter((f) => !f.enabled && f.sitemapExclude)
	.map((f) => f.sitemapExclude!);
const siteUrl = siteConfig.url;
const vitePublicDefines = Object.fromEntries(
	Object.entries(publicEnv).map(([key, value]) => [`import.meta.env.${key}`, JSON.stringify(value)])
);

if (featureFlags.viewCounter.enabled && !publicEnv.PUBLIC_VIEW_COUNTER_URL) {
	throw new Error('PUBLIC_VIEW_COUNTER_URL is required when PUBLIC_FEATURE_VIEW_COUNTER=true.');
}

if (featureFlags.contact.enabled) {
	if (!publicEnv.PUBLIC_CONTACT_URL) {
		throw new Error('PUBLIC_CONTACT_URL is required when PUBLIC_FEATURE_CONTACT=true.');
	}

	if (!publicEnv.PUBLIC_TURNSTILE_SITE_KEY) {
		throw new Error('PUBLIC_TURNSTILE_SITE_KEY is required when PUBLIC_FEATURE_CONTACT=true.');
	}
}

const prettyCodeOptions = {
	onVisitLine(node: { children: Array<{ type: string; value: string }> }) {
		if (node.children.length === 0) {
			node.children = [{ type: 'text', value: ' ' }];
		}
	},
	onVisitHighlightedLine(node: { properties: { className?: string[] } }) {
		node.properties.className = ['highlighted'];
	}
};

function featureRoutesIntegration(): AstroIntegration {
	return {
		name: 'feature-routes',
		hooks: {
			'astro:config:setup': ({ injectRoute }) => {
				for (const feature of blogFeatures) {
					if (!feature.enabled || !feature.routes) continue;
					for (const route of feature.routes) {
						injectRoute(route);
					}
				}
			}
		}
	};
}

export default defineConfig({
	site: siteUrl,

	markdown: {
		syntaxHighlight: false,
		rehypePlugins: [[rehypePrettyCode, prettyCodeOptions]],
		remarkPlugins: [remarkReadingTime]
	},

	integrations: [
		featureRoutesIntegration(),
		...featureIntegrations,
		sitemap({
			filter(page) {
				const pathname = typeof page === 'string' ? new URL(page).pathname : '';
				for (const exclude of sitemapExclusions) {
					if (exclude(pathname)) return false;
				}
				return true;
			}
		})
	],

	output: 'static',

	vite: {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		plugins: [tailwindcss() as any],
		define: vitePublicDefines,
		ssr: {
			external: ['@resvg/resvg-js', 'satori', 'node:fs', 'node:path']
		}
	},

	adapter: cloudflare()
});
