import type { APIRoute, GetStaticPaths } from 'astro';
import satori from 'satori';
import { Resvg } from '@resvg/resvg-js';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { getCollection, type CollectionEntry } from 'astro:content';
import { siteConfig } from '@/config/site.config';
import { getTagDisplay } from '@/lib/format/tagDisplay';

const fontBuf = readFileSync(join(process.cwd(), 'src/assets/fonts/JetBrainsMono-Bold.ttf'));
const fontData = fontBuf.buffer.slice(
	fontBuf.byteOffset,
	fontBuf.byteOffset + fontBuf.byteLength
) as ArrayBuffer;

type SatoriNode = {
	type: string;
	props: {
		style?: Record<string, string | number>;
		children?: Array<SatoriNode | string> | SatoriNode | string;
	};
};

function el(
	type: string,
	style: Record<string, string | number>,
	children: Array<SatoriNode | string> = []
): SatoriNode {
	const nextStyle = type === 'div' && style.display == null ? { ...style, display: 'flex' } : style;
	return { type, props: { style: nextStyle, children } };
}

export const getStaticPaths: GetStaticPaths = async () => {
	const posts: CollectionEntry<'posts'>[] = await getCollection('posts');
	return posts.map((entry) => ({
		params: { slug: entry.slug },
		props: {
			title: entry.data.title,
			tags: entry.data.tags ?? []
		}
	}));
};

export const GET: APIRoute = async ({ props }) => {
	const { title, tags } = props as { title: string; tags: string[] };
	const fontSize = title.length > 55 ? 50 : title.length > 38 ? 58 : 66;

	const svg = await satori(buildCard(title, tags, fontSize), {
		width: 1200,
		height: 630,
		fonts: [{ name: 'JetBrains Mono', data: fontData, weight: 700, style: 'normal' }]
	});

	const png = new Uint8Array(new Resvg(svg).render().asPng());

	return new Response(png, {
		headers: {
			'Content-Type': 'image/png',
			'Cache-Control': 'public, max-age=31536000, immutable'
		}
	});
};

function buildCard(title: string, tags: string[], fontSize: number): SatoriNode {
	const palette = siteConfig.og.palette;
	const brand = siteConfig.brand;

	return el(
		'div',
		{
			width: '1200px',
			height: '630px',
			display: 'flex',
			backgroundColor: palette.bg,
			fontFamily: 'JetBrains Mono',
			position: 'relative',
			overflow: 'hidden'
		},
		[
			el('div', {
				position: 'absolute',
				left: 0,
				top: 0,
				width: '6px',
				height: '630px',
				background: palette.rule
			}),
			el(
				'div',
				{
					display: 'flex',
					flexDirection: 'column',
					flex: 1,
					padding: '56px 72px 56px 80px'
				},
				[
					el('div', { display: 'flex', alignItems: 'center' }, [
						el('span', { color: palette.brandSoft, fontSize: 20, letterSpacing: '0.1em' }, [
							brand.name
						]),
						...(brand.tld
							? [
									el('span', { color: palette.brandStrong, fontSize: 20 }, ['.']),
									el('span', { color: palette.brandSoft, fontSize: 20, letterSpacing: '0.1em' }, [
										brand.tld
									])
								]
							: [])
					]),
					el('div', { display: 'flex', flex: 1, alignItems: 'center' }, [
						el(
							'div',
							{
								color: palette.title,
								fontSize,
								lineHeight: 1.2,
								fontWeight: 700,
								maxWidth: '1040px'
							},
							[title]
						)
					]),
					el(
						'div',
						{
							display: 'flex',
							justifyContent: 'space-between',
							alignItems: 'center'
						},
						[
							el(
								'div',
								{ display: 'flex', gap: '10px' },
								tags.slice(0, 3).map((tag) =>
									el(
										'span',
										{
											backgroundColor: palette.tagBg,
											color: palette.tagText,
											padding: '4px 14px',
											borderRadius: '4px',
											fontSize: 18,
											letterSpacing: '0.03em'
										},
										[getTagDisplay(tag)]
									)
								)
							),
							el('div', { color: palette.subtitle, fontSize: 18 }, [siteConfig.author.name])
						]
					)
				]
			)
		]
	);
}
