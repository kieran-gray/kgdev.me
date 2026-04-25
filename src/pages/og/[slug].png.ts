import type { APIRoute, GetStaticPaths, MarkdownInstance } from 'astro';
import satori from 'satori';
import { Resvg } from '@resvg/resvg-js';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import React from 'react';
import { AppConfig } from '@/utils/AppConfig';

const fontBuf = readFileSync(join(process.cwd(), 'src/assets/fonts/JetBrainsMono-Bold.ttf'));
const fontData = fontBuf.buffer.slice(fontBuf.byteOffset, fontBuf.byteOffset + fontBuf.byteLength) as ArrayBuffer;

export const getStaticPaths: GetStaticPaths = async () => {
	const posts = Object.entries(
		import.meta.glob('../posts/*.md', { eager: true })
	) as [string, MarkdownInstance<Record<string, unknown>>][];

	return posts.map(([filePath, post]) => {
		const slug = filePath.split('/').pop()!.replace('.md', '');
		return {
			params: { slug },
			props: {
				title: post.frontmatter['title'] as string,
				tags: (post.frontmatter['tags'] as string[] | undefined) ?? [],
			},
		};
	});
};

export const GET: APIRoute = async ({ props }) => {
	const { title, tags } = props as { title: string; tags: string[] };
	const fontSize = title.length > 55 ? 50 : title.length > 38 ? 58 : 66;

	const svg = await satori(buildCard(title, tags, fontSize), {
		width: 1200,
		height: 630,
		fonts: [{ name: 'JetBrains Mono', data: fontData, weight: 700, style: 'normal' }],
	});

	const png = new Uint8Array(new Resvg(svg).render().asPng());

	return new Response(png, {
		headers: {
			'Content-Type': 'image/png',
			'Cache-Control': 'public, max-age=31536000, immutable',
		},
	});
};

function buildCard(title: string, tags: string[], fontSize: number) {
	return React.createElement(
		'div',
		{
			style: {
				width: '1200px',
				height: '630px',
				display: 'flex',
				backgroundColor: '#0f1115',
				fontFamily: '"JetBrains Mono"',
				position: 'relative',
				overflow: 'hidden',
			},
		},
		React.createElement('div', {
			style: {
				position: 'absolute',
				left: 0,
				top: 0,
				width: '6px',
				height: '630px',
				background: '#e25c72',
			},
		}),
		React.createElement(
			'div',
			{
				style: {
					display: 'flex',
					flexDirection: 'column',
					flex: 1,
					padding: '56px 72px 56px 80px',
				},
			},
			React.createElement(
				'div',
				{ style: { display: 'flex', alignItems: 'center' } },
				React.createElement('span', { style: { color: '#fda4af', fontSize: 20, letterSpacing: '0.1em' } }, 'KGDEV'),
				React.createElement('span', { style: { color: '#e25c72', fontSize: 20 } }, '.'),
				React.createElement('span', { style: { color: '#fda4af', fontSize: 20, letterSpacing: '0.1em' } }, 'me')
			),
			React.createElement(
				'div',
				{ style: { display: 'flex', flex: 1, alignItems: 'center' } },
				React.createElement('div', {
					style: {
						color: '#f3f4f6',
						fontSize,
						lineHeight: 1.2,
						fontWeight: 700,
						maxWidth: '1040px',
					},
					children: title,
				})
			),
			React.createElement(
				'div',
				{
					style: {
						display: 'flex',
						justifyContent: 'space-between',
						alignItems: 'center',
					},
				},
				React.createElement(
					'div',
					{ style: { display: 'flex', gap: '10px' } },
					...tags.slice(0, 3).map((tag) =>
						React.createElement(
							'span',
							{
								key: tag,
								style: {
									backgroundColor: '#5b1f2d',
									color: '#fecdd3',
									padding: '4px 14px',
									borderRadius: '4px',
									fontSize: 18,
									letterSpacing: '0.03em',
								},
							},
							tag
						)
					)
				),
				React.createElement(
					'div',
					{ style: { color: '#9ca3af', fontSize: 18 } },
					AppConfig.author
				)
			)
		)
	);
}
