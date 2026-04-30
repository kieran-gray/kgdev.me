import { blogConfig } from '../../blog.config';

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

export interface BrandConfig {
	name: string;
	tld?: string;
	accentDot: boolean;
	favicon?: string;
}

export interface SiteAuthor {
	name: string;
	jobTitle: string;
	bio: string;
}

export interface SiteMeta {
	title: string;
	description: string;
	locale: string;
}

export interface SiteConfig {
	url: string;
	brand: BrandConfig;
	meta: SiteMeta;
	author: SiteAuthor;
	social: SocialLink[];
	nav: NavItem[];
	ogTagline: string;
	ogFallbackImage?: string;
}

export const siteConfig: SiteConfig = blogConfig.site;
