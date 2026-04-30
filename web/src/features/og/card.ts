import satori from 'satori';
import { Resvg } from '@resvg/resvg-js';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { siteConfig } from '@/config/site';
import { ogPalette, type OgPalette } from '@/config/theme';
import { getTagDisplay } from '@/lib/format/tagDisplay';

const fontBuf = readFileSync(join(process.cwd(), 'src/assets/fonts/JetBrainsMono-Bold.ttf'));
const fontData = fontBuf.buffer.slice(
	fontBuf.byteOffset,
	fontBuf.byteOffset + fontBuf.byteLength
) as ArrayBuffer;

export type SatoriNode = {
	type: string;
	props: {
		style?: Record<string, string | number>;
		children?: Array<SatoriNode | string> | SatoriNode | string;
	};
};

export function el(
	type: string,
	style: Record<string, string | number>,
	children: Array<SatoriNode | string> = []
): SatoriNode {
	const nextStyle = type === 'div' && style.display == null ? { ...style, display: 'flex' } : style;
	return { type, props: { style: nextStyle, children } };
}

function brandRow(palette: OgPalette): SatoriNode {
	const { brand } = siteConfig;
	return el('div', { display: 'flex', alignItems: 'center' }, [
		el('span', { color: palette.brandSoft, fontSize: 20, letterSpacing: '0.1em' }, [brand.name]),
		...(brand.tld
			? [
					el('span', { color: palette.brandStrong, fontSize: 20 }, ['.']),
					el('span', { color: palette.brandSoft, fontSize: 20, letterSpacing: '0.1em' }, [
						brand.tld
					])
				]
			: [])
	]);
}

function frame(palette: OgPalette, body: SatoriNode): SatoriNode {
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
			body
		]
	);
}

function buildPostCard(title: string, tags: string[]): SatoriNode {
	const palette = ogPalette;
	const fontSize = title.length > 55 ? 50 : title.length > 38 ? 58 : 66;

	return frame(
		palette,
		el(
			'div',
			{
				display: 'flex',
				flexDirection: 'column',
				flex: 1,
				padding: '56px 72px 56px 80px'
			},
			[
				brandRow(palette),
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
	);
}

function buildDefaultCard(): SatoriNode {
	const palette = ogPalette;
	const domain = siteConfig.url.replace(/^https?:\/\//, '');

	return frame(
		palette,
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
				brandRow(palette),
				el('div', { display: 'flex', flexDirection: 'column', gap: '16px' }, [
					el('div', { color: palette.title, fontSize: 72, fontWeight: 700, lineHeight: 1.1 }, [
						siteConfig.author.name
					]),
					el('div', { color: palette.subtitle, fontSize: 28, lineHeight: 1.4 }, [
						siteConfig.ogTagline
					])
				]),
				el('div', { color: palette.caption, fontSize: 18 }, [domain])
			]
		)
	);
}

async function renderPng(node: SatoriNode): Promise<Uint8Array<ArrayBuffer>> {
	const svg = await satori(node, {
		width: 1200,
		height: 630,
		fonts: [{ name: 'JetBrains Mono', data: fontData, weight: 700, style: 'normal' }]
	});
	const buf = new Resvg(svg).render().asPng();
	const ab = new ArrayBuffer(buf.byteLength);
	new Uint8Array(ab).set(buf);
	return new Uint8Array(ab);
}

export async function renderPostCard(title: string, tags: string[]) {
	return renderPng(buildPostCard(title, tags));
}

export async function renderDefaultCard() {
	return renderPng(buildDefaultCard());
}
