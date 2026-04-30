import type { SiteConfig } from './src/config/site';
import type { ThemeConfigInput } from './src/config/theme';

export interface BlogConfig {
	site: SiteConfig;
	theme: ThemeConfigInput;
}

/**
 * Customization entrypoint for the blog template.
 */
export const blogConfig: BlogConfig = {
	site: {
		url: 'https://kgdev.me',
		brand: {
			name: 'KGDEV',
			tld: 'me',
			accentDot: true,
			favicon:
				'data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 100 100%22><text y=%22.9em%22 font-size=%2290%22>👨‍💻</text></svg>'
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
			{
				kind: 'github',
				label: 'GitHub social link',
				href: 'https://github.com/kieran-gray'
			},
			{
				kind: 'linkedin',
				label: 'Linkedin social link',
				href: 'https://www.linkedin.com/in/kieran-g'
			},
			{ kind: 'email', label: 'Email address', href: 'mailto:gray.kieran@protonmail.com' }
		],
		nav: [
			{ label: 'HOME', href: '/' },
			{ label: 'BLOG', href: '/posts' },
			{ label: 'PROJECTS', href: '/projects' },
			{ label: 'BOOKS', href: '/books' }
		],
		ogTagline: 'Rust · TypeScript · Distributed Systems · Cloudflare'
	},
	theme: {
		default: 'rose'
	}
};
