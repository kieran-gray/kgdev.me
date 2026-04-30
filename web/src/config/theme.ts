import { schemes, SCHEME_NAMES, type SchemeName, type ThemeTokens } from '@/styles/schemes';
import type { SocialKind } from './site';
import { blogConfig } from '../../blog.config';

export { schemes, SCHEME_NAMES };
export type { SchemeName, ThemeTokens };

export interface ThemeConfigInput {
	default: SchemeName;
	list?: readonly SchemeName[];
}

export interface ThemeConfig {
	default: SchemeName;
	list: readonly SchemeName[];
}

export const theme: ThemeConfig = {
	default: blogConfig.theme.default,
	list: blogConfig.theme.list ?? SCHEME_NAMES
};

export interface OgPalette {
	bg: string;
	rule: string;
	brandSoft: string;
	brandStrong: string;
	title: string;
	subtitle: string;
	caption: string;
	tagBg: string;
	tagText: string;
}

/**
 * Og card palette is derived from the theme tokens, but can be customized if needed.
 */
export function deriveOgPalette(tokens: ThemeTokens): OgPalette {
	return {
		bg: tokens.pageBg,
		rule: tokens.accent700,
		brandSoft: tokens.accent500,
		brandStrong: tokens.accent700,
		title: tokens.textColor,
		subtitle: tokens.mutedText,
		caption: tokens.mutedText,
		tagBg: tokens.tagBg,
		tagText: tokens.badgeText
	};
}

export const ogPalette: OgPalette = deriveOgPalette(schemes[theme.default].dark);

const ICON_GITHUB =
	'<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="currentColor" viewBox="0 0 24 24"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>';

const ICON_LINKEDIN =
	'<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="currentColor" viewBox="0 0 24 24"><path d="M4.98 3.5c0 1.381-1.11 2.5-2.48 2.5s-2.48-1.119-2.48-2.5c0-1.38 1.11-2.5 2.48-2.5s2.48 1.12 2.48 2.5zm.02 4.5h-5v16h5v-16zm7.982 0h-4.968v16h4.969v-8.399c0-4.67 6.029-5.052 6.029 0v8.399h4.988v-10.131c0-7.88-8.922-7.593-11.018-3.714v-2.155z"/></svg>';

const ICON_TWITTER =
	'<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="currentColor" viewBox="0 0 24 24"><path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"/></svg>';

const ICON_MASTODON =
	'<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="currentColor" viewBox="0 0 24 24"><path d="M23.193 7.88c0-5.207-3.411-6.733-3.411-6.733C18.062.357 15.108.025 12.041 0h-.076c-3.068.025-6.02.357-7.74 1.147 0 0-3.412 1.526-3.412 6.733 0 1.193-.023 2.619.015 4.13.124 5.092.934 10.11 5.641 11.355 2.17.574 4.034.695 5.535.612 2.722-.151 4.25-.972 4.25-.972l-.09-1.975s-1.945.613-4.129.539c-2.165-.074-4.448-.233-4.798-2.892a5.5 5.5 0 0 1-.05-.745s2.124.519 4.818.642c1.647.075 3.192-.097 4.762-.283 3.01-.36 5.63-2.218 5.96-3.916.52-2.673.477-6.521.477-6.521zm-4.024 6.716h-2.498V8.469c0-1.29-.543-1.944-1.628-1.944-1.2 0-1.802.776-1.802 2.312v3.349h-2.484V8.838c0-1.536-.602-2.312-1.802-2.312-1.085 0-1.628.655-1.628 1.944v6.127H4.829V8.285c0-1.289.328-2.313.987-3.07.68-.758 1.569-1.146 2.674-1.146 1.278 0 2.246.491 2.886 1.474L12 6.585l.624-1.042c.64-.983 1.608-1.474 2.886-1.474 1.104 0 1.994.388 2.674 1.146.658.757.985 1.781.985 3.07z"/></svg>';

const ICON_EMAIL =
	'<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="currentColor" viewBox="0 0 24 24"><path d="M0 3v18h24V3H0zm21.518 2L12 12.713 2.482 5h19.036zM2 19V7.183l10 8.104 10-8.104V19H2z"/></svg>';

const ICON_RSS =
	'<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="currentColor" viewBox="0 0 24 24"><path d="M6.18 15.64a2.18 2.18 0 0 1 2.18 2.18C8.36 19 7.38 20 6.18 20A2.18 2.18 0 0 1 4 17.82a2.18 2.18 0 0 1 2.18-2.18M4 4.44A15.56 15.56 0 0 1 19.56 20h-2.83A12.73 12.73 0 0 0 4 7.27V4.44m0 5.66a9.9 9.9 0 0 1 9.9 9.9h-2.83A7.07 7.07 0 0 0 4 12.93V10.1z"/></svg>';

export const socialIcons: Record<SocialKind, string> = {
	github: ICON_GITHUB,
	linkedin: ICON_LINKEDIN,
	twitter: ICON_TWITTER,
	mastodon: ICON_MASTODON,
	email: ICON_EMAIL,
	rss: ICON_RSS
};
