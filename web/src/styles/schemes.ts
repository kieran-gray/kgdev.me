export const SCHEME_NAMES = ['amber', 'emerald', 'indigo', 'rose'] as const;
export type SchemeName = (typeof SCHEME_NAMES)[number];

export interface ThemeTokens {
	accent700: string;
	accent500: string;
	pageBg: string;
	cardOuterBg: string;
	cardInnerBg: string;
	textColor: string;
	link: string;
	linkHover: string;
	mutedText: string;
	borderColor: string;
	tagBg: string;
	tagBgHover: string;
	tagText: string;
	badgeBg: string;
	badgeText: string;
	codeBg: string;
	codeInlineBg: string;
}

export interface Scheme {
	swatch: string;
	label: string;
	light: ThemeTokens;
	dark: ThemeTokens;
}

const lightShared = {
	pageBg: '#f5f5f4',
	cardInnerBg: '#ffffff',
	textColor: '#111827',
	mutedText: '#515761',
	borderColor: '#e5e7eb',
	tagText: '#ffffff',
	codeBg: '#1f2937',
	codeInlineBg: '#ebebeb'
} as const;

const darkShared = {
	pageBg: '#0f1115',
	cardOuterBg: '#151a1f',
	cardInnerBg: '#1c2128',
	textColor: '#e5e7eb',
	mutedText: '#aab0bb',
	borderColor: '#2a2f36',
	tagText: '#ffffff',
	codeBg: '#11161c',
	codeInlineBg: '#11161c'
} as const;

export const schemes: Record<SchemeName, Scheme> = {
	amber: {
		swatch: '#f59e0b',
		label: 'Amber',
		light: {
			...lightShared,
			accent700: '#b45309',
			accent500: '#f59e0b',
			cardOuterBg: 'rgba(254, 243, 199, 0.5)',
			link: '#b45309',
			linkHover: '#92400e',
			tagBg: '#b45309',
			tagBgHover: '#d97706',
			badgeBg: '#fef3c7',
			badgeText: '#92400e'
		},
		dark: {
			...darkShared,
			accent700: '#eab308',
			accent500: '#facc15',
			link: '#facc15',
			linkHover: '#fde68a',
			tagBg: '#4a3410',
			tagBgHover: '#5a3e12',
			badgeBg: '#2b1f08',
			badgeText: '#f1d48a'
		}
	},
	emerald: {
		swatch: '#10b981',
		label: 'Emerald',
		light: {
			...lightShared,
			accent700: '#047857',
			accent500: '#10b981',
			cardOuterBg: 'rgba(209, 250, 229, 0.7)',
			link: '#047857',
			linkHover: '#065f46',
			tagBg: '#047857',
			tagBgHover: '#065f46',
			badgeBg: '#d1fae5',
			badgeText: '#065f46'
		},
		dark: {
			...darkShared,
			accent700: '#22a36b',
			accent500: '#34d399',
			link: '#34d399',
			linkHover: '#86efac',
			tagBg: '#0f3f2e',
			tagBgHover: '#145a3a',
			badgeBg: '#0c2f24',
			badgeText: '#a7f3d0'
		}
	},
	indigo: {
		swatch: '#6366f1',
		label: 'Indigo',
		light: {
			...lightShared,
			accent700: '#4338ca',
			accent500: '#6366f1',
			cardOuterBg: 'rgba(224, 231, 255, 0.7)',
			link: '#4338ca',
			linkHover: '#3730a3',
			tagBg: '#4338ca',
			tagBgHover: '#3730a3',
			badgeBg: '#e0e7ff',
			badgeText: '#3730a3'
		},
		dark: {
			...darkShared,
			accent700: '#6f76e4',
			accent500: '#a5b4fc',
			link: '#a5b4fc',
			linkHover: '#c7d2fe',
			tagBg: '#2c2f6b',
			tagBgHover: '#3a3e8a',
			badgeBg: '#20244d',
			badgeText: '#c7d2fe'
		}
	},
	rose: {
		swatch: '#f43f5e',
		label: 'Rose',
		light: {
			...lightShared,
			accent700: '#be123c',
			accent500: '#f43f5e',
			cardOuterBg: 'rgba(255, 228, 230, 0.7)',
			link: '#be123c',
			linkHover: '#9f1239',
			tagBg: '#be123c',
			tagBgHover: '#e11d48',
			badgeBg: '#ffe4e6',
			badgeText: '#9f1239'
		},
		dark: {
			...darkShared,
			accent700: '#e25c72',
			accent500: '#fda4af',
			link: '#fda4af',
			linkHover: '#fecdd3',
			tagBg: '#5b1f2d',
			tagBgHover: '#7a2537',
			badgeBg: '#4a1520',
			badgeText: '#fecdd3'
		}
	}
};

const tokenMap: Record<keyof ThemeTokens, string> = {
	accent700: '--accent-700',
	accent500: '--accent-500',
	pageBg: '--page-bg',
	cardOuterBg: '--card-outer-bg',
	cardInnerBg: '--card-inner-bg',
	textColor: '--text-color',
	link: '--link',
	linkHover: '--link-hover',
	mutedText: '--muted-text',
	borderColor: '--border-color',
	tagBg: '--tag-bg',
	tagBgHover: '--tag-bg-hover',
	tagText: '--tag-text',
	badgeBg: '--badge-bg',
	badgeText: '--badge-text',
	codeBg: '--code-bg',
	codeInlineBg: '--code-inline-bg'
};

function tokensToCss(tokens: ThemeTokens): string {
	return (Object.keys(tokenMap) as (keyof ThemeTokens)[])
		.map((key) => `${tokenMap[key]}: ${tokens[key]};`)
		.join(' ');
}

/**
 * Generates token CSS for every scheme. Default scheme is also written
 * to bare `:root` / `:root.dark` so unscoped pages render correctly
 * before any data-scheme attribute is applied.
 */
export function generateSchemeCss(defaultScheme: SchemeName): string {
	const blocks: string[] = [];
	const def = schemes[defaultScheme];

	blocks.push(`:root { ${tokensToCss(def.light)} }`);
	blocks.push(`:root.dark { ${tokensToCss(def.dark)} }`);

	for (const name of SCHEME_NAMES) {
		const s = schemes[name];
		blocks.push(`:root[data-scheme='${name}'] { ${tokensToCss(s.light)} }`);
		blocks.push(`:root.dark[data-scheme='${name}'] { ${tokensToCss(s.dark)} }`);
	}

	return blocks.join('\n');
}
