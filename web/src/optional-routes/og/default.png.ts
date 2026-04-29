import type { APIRoute } from 'astro';
import satori from 'satori';
import { Resvg } from '@resvg/resvg-js';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { siteConfig } from '@/config/site.config';

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

function buildDefaultCard(): SatoriNode {
	const palette = siteConfig.og.palette;
	const brand = siteConfig.brand;
	const domain = siteConfig.url.replace(/^https?:\/\//, '');

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
					padding: '56px 72px 56px 80px',
					justifyContent: 'space-between'
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
					el('div', { display: 'flex', flexDirection: 'column', gap: '16px' }, [
						el('div', { color: palette.title, fontSize: 72, fontWeight: 700, lineHeight: 1.1 }, [
							siteConfig.author.name
						]),
						el('div', { color: palette.subtitle, fontSize: 28, lineHeight: 1.4 }, [
							siteConfig.og.tagline
						])
					]),
					el('div', { color: palette.caption, fontSize: 18 }, [domain])
				]
			)
		]
	);
}
