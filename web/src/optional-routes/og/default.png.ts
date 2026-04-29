import type { APIRoute } from 'astro';
import satori from 'satori';
import { Resvg } from '@resvg/resvg-js';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import React from 'react';
import { siteConfig } from '@/config/site.config';

const fontBuf = readFileSync(join(process.cwd(), 'src/assets/fonts/JetBrainsMono-Bold.ttf'));
const fontData = fontBuf.buffer.slice(
	fontBuf.byteOffset,
	fontBuf.byteOffset + fontBuf.byteLength
) as ArrayBuffer;

export const GET: APIRoute = async () => {
	const svg = await satori(buildDefaultCard(), {
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

function buildDefaultCard() {
	const palette = siteConfig.og.palette;
	const brand = siteConfig.brand;
	const domain = siteConfig.url.replace(/^https?:\/\//, '');

	return React.createElement(
		'div',
		{
			style: {
				width: '1200px',
				height: '630px',
				display: 'flex',
				backgroundColor: palette.bg,
				fontFamily: '"JetBrains Mono"',
				position: 'relative',
				overflow: 'hidden'
			}
		},
		React.createElement('div', {
			style: {
				position: 'absolute',
				left: 0,
				top: 0,
				width: '6px',
				height: '630px',
				background: palette.rule
			}
		}),
		React.createElement(
			'div',
			{
				style: {
					display: 'flex',
					flexDirection: 'column',
					flex: 1,
					padding: '56px 72px 56px 80px',
					justifyContent: 'space-between'
				}
			},
			React.createElement(
				'div',
				{ style: { display: 'flex', alignItems: 'center' } },
				React.createElement(
					'span',
					{ style: { color: palette.brandSoft, fontSize: 20, letterSpacing: '0.1em' } },
					brand.name
				),
				brand.tld &&
					React.createElement('span', { style: { color: palette.brandStrong, fontSize: 20 } }, '.'),
				brand.tld &&
					React.createElement(
						'span',
						{ style: { color: palette.brandSoft, fontSize: 20, letterSpacing: '0.1em' } },
						brand.tld
					)
			),
			React.createElement(
				'div',
				{ style: { display: 'flex', flexDirection: 'column', gap: '16px' } },
				React.createElement('div', {
					style: { color: palette.title, fontSize: 72, fontWeight: 700, lineHeight: 1.1 },
					children: siteConfig.author.name
				}),
				React.createElement('div', {
					style: { color: palette.subtitle, fontSize: 28, lineHeight: 1.4 },
					children: siteConfig.og.tagline
				})
			),
			React.createElement('div', {
				style: { color: palette.caption, fontSize: 18 },
				children: domain
			})
		)
	);
}
