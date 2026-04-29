import type { SchemeName } from '@/styles/schemes';
import { features } from './features.mjs';

export type SocialKind = 'github' | 'linkedin' | 'twitter' | 'mastodon' | 'email' | 'rss';

export interface SocialLink {
	kind: SocialKind;
	label: string;
	href: string;
}

export interface NavItem {
	label: string;
	href: string;
}

export interface CurrentRole {
	title: string;
	org?: string;
	orgUrl?: string;
	summary?: string;
}

export interface HeroParagraph {
	text: string;
	highlights?: string[];
}

export interface HeroConfig {
	headline: string;
	paragraphs: HeroParagraph[];
	currentRole?: CurrentRole;
}

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

export interface OgConfig {
	tagline: string;
	palette: OgPalette;
	fallbackImage?: string;
}

export interface SiteFeatures {
	search: { enabled: boolean };
	viewCounter: { enabled: boolean; wsUrl: string };
	contact: { enabled: boolean; endpoint: string; turnstileSiteKey: string };
	og: { enabled: boolean };
	mermaid: { enabled: boolean };
	rss: { enabled: boolean };
}

export interface BrandConfig {
	name: string;
	tld?: string;
	accentDot: boolean;
}

export interface SiteAuthor {
	name: string;
	jobTitle: string;
	bio: string;
}

export interface SiteSchemes {
	default: SchemeName;
	list: readonly SchemeName[];
}

export interface SiteConfig {
	url: string;
	brand: BrandConfig;
	meta: { title: string; description: string; locale: string };
	author: SiteAuthor;
	social: SocialLink[];
	nav: NavItem[];
	features: SiteFeatures;
	hero: HeroConfig;
	og: OgConfig;
	schemes: SiteSchemes;
}

export const siteConfig: SiteConfig = {
	url: 'https://kgdev.me',
	brand: {
		name: 'KGDEV',
		tld: 'me',
		accentDot: true
	},
	meta: {
		title: 'Kieran Gray',
		description:
			'Software engineer writing about things that interest me. Deep dives into distributed systems and real-world architecture.',
		locale: 'en'
	},
	author: {
		name: 'Kieran Gray',
		jobTitle: 'Software Engineer',
		bio: 'Software engineer with 5 years of industry experience building scalable backend systems and full-stack applications.'
	},
	social: [
		{ kind: 'github', label: 'GitHub social link', href: 'https://github.com/kieran-gray' },
		{
			kind: 'linkedin',
			label: 'Linkedin social link',
			href: 'https://www.linkedin.com/in/kieran-g'
		}
	],
	nav: [
		{ label: 'HOME', href: '/' },
		{ label: 'BLOG', href: '/posts' },
		{ label: 'PROJECTS', href: '/projects' },
		{ label: 'BOOKS', href: '/books' }
	],
	features: {
		search: { enabled: features.search.enabled },
		viewCounter: {
			enabled: features.viewCounter.enabled,
			wsUrl: import.meta.env.PUBLIC_VIEW_COUNTER_URL ?? 'wss://counter.kgdev.me'
		},
		contact: {
			enabled: features.contact.enabled,
			endpoint: import.meta.env.PUBLIC_CONTACT_URL ?? 'https://contact.kgdev.me/api/v1/contact/',
			turnstileSiteKey: import.meta.env.PUBLIC_TURNSTILE_SITE_KEY ?? '0x4AAAAAADFJ-aGGTpoBJuyq'
		},
		og: { enabled: features.og.enabled },
		mermaid: { enabled: features.mermaid.enabled },
		rss: { enabled: features.rss.enabled }
	},
	hero: {
		headline: 'KIERAN GRAY',
		paragraphs: [
			{ text: 'I’m a software engineer with 5 years of industry experience.' },
			{
				text: 'I build scalable backend systems and full‑stack applications across {0}, {1}, {2}, and {3}.',
				highlights: ['telecom', 'geospatial', 'SaaS', 'insurance']
			},
			{
				text: 'I’ve worked primarily with {0}, {1}, and {2}, with a strong focus on clean architecture, developer experience, and cloud infrastructure.',
				highlights: ['Python', 'TypeScript', 'Rust']
			},
			{
				text: 'Read recent posts for deep dives, browse projects to see what I’m building, or connect via the links below.'
			}
		],
		currentRole: {
			title: 'Software Engineer',
			org: 'Prima Assicurazioni',
			orgUrl: 'https://helloprima.com',
			summary:
				'Building Elixir services (plus a bit of Elm) in insurance. Sharpening my skills in Elixir, event sourcing, and DDD as I go.'
		}
	},
	og: {
		tagline: 'Rust · TypeScript · Distributed Systems · Cloudflare',
		palette: {
			bg: '#0f1115',
			rule: '#e25c72',
			brandSoft: '#fda4af',
			brandStrong: '#e25c72',
			title: '#f3f4f6',
			subtitle: '#9ca3af',
			caption: '#6b7280',
			tagBg: '#5b1f2d',
			tagText: '#fecdd3'
		}
	},
	schemes: {
		default: 'rose',
		list: ['amber', 'emerald', 'indigo', 'rose'] as const
	}
};
