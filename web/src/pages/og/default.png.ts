import type { APIRoute } from 'astro';
import satori from 'satori';
import { Resvg } from '@resvg/resvg-js';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import React from 'react';
import { AppConfig } from '@/utils/AppConfig';

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
				background: '#e25c72'
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
					{ style: { color: '#fda4af', fontSize: 20, letterSpacing: '0.1em' } },
					'KGDEV'
				),
				React.createElement('span', { style: { color: '#e25c72', fontSize: 20 } }, '.'),
				React.createElement(
					'span',
					{ style: { color: '#fda4af', fontSize: 20, letterSpacing: '0.1em' } },
					'me'
				)
			),
			React.createElement(
				'div',
				{ style: { display: 'flex', flexDirection: 'column', gap: '16px' } },
				React.createElement('div', {
					style: { color: '#f3f4f6', fontSize: 72, fontWeight: 700, lineHeight: 1.1 },
					children: AppConfig.author
				}),
				React.createElement('div', {
					style: { color: '#9ca3af', fontSize: 28, lineHeight: 1.4 },
					children: 'Rust · TypeScript · Distributed Systems · Cloudflare'
				})
			),
			React.createElement('div', {
				style: { color: '#6b7280', fontSize: 18 },
				children: 'kgdev.me'
			})
		)
	);
}
