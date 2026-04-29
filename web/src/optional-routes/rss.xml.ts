import type { APIRoute } from 'astro';
import rss from '@astrojs/rss';
import { getCollection } from 'astro:content';
import { siteConfig } from '@/config/site.config';

export const GET: APIRoute = async (context) => {
	const posts = await getCollection('posts');
	return rss({
		title: siteConfig.meta.title,
		description: siteConfig.meta.description,
		site: context.site ?? siteConfig.url,
		items: posts
			.sort((a, b) => b.data.pubDate.getTime() - a.data.pubDate.getTime())
			.map((post) => ({
				title: post.data.title,
				description: post.data.description,
				pubDate: post.data.pubDate,
				link: `/posts/${post.slug}/`
			}))
	});
};
