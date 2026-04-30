#!/usr/bin/env node
import { createInterface } from 'readline';
import { writeFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..');

const rl = createInterface({ input: process.stdin, output: process.stdout });

const ask = (question, defaultValue) =>
	new Promise((resolve) => {
		const hint = defaultValue !== undefined ? ` (default: ${defaultValue})` : '';
		rl.question(`${question}${hint}: `, (answer) => {
			resolve(answer.trim() || defaultValue || '');
		});
	});

const askYN = (question, defaultValue = true) =>
	new Promise((resolve) => {
		const hint = defaultValue ? ' [Y/n]' : ' [y/N]';
		rl.question(`${question}${hint}: `, (answer) => {
			const a = answer.trim().toLowerCase();
			if (!a) resolve(defaultValue);
			else resolve(a === 'y' || a === 'yes');
		});
	});

const hr = () => console.log('\n' + '─'.repeat(60));
const section = (title) => { hr(); console.log(`  ${title}`); hr(); console.log(); };

console.log('\n╔══════════════════════════════════════════════════════════╗');
console.log('║              Blog Template Setup                        ║');
console.log('╚══════════════════════════════════════════════════════════╝\n');
console.log('This script will configure blog.config.ts, wrangler.jsonc,');
console.log('and your homepage copy. Press Enter to accept defaults.\n');

// ─── Site ────────────────────────────────────────────────────────────────────

section('SITE');

const siteUrl = await ask('Site URL', 'https://example.com');
const brandName = await ask('Brand name (shown in nav / header)', 'MYBLOG');
const brandTld = await ask('Brand TLD suffix (e.g. "com" for MYBLOG.com, leave blank to omit)', '');
const accentDot = await askYN('Show accent dot between brand name and TLD?', true);

// ─── Author ──────────────────────────────────────────────────────────────────

section('AUTHOR');

const authorName = await ask('Your full name', 'Your Name');
const pageTitle = await ask('HTML page title (shown in browser tab)', authorName);
const jobTitle = await ask('Job title', 'Software Engineer');
const metaDescription = await ask(
	'Meta description (1–2 sentences for search engines)',
	`${authorName}'s personal blog.`
);
const authorBio = await ask(
	'Short author bio (used in structured data)',
	`${jobTitle} and writer.`
);
const ogTagline = await ask(
	'OG tagline (short · dot-separated · shown on social cards)',
	'Writing · Building · Learning'
);

// ─── Social links ─────────────────────────────────────────────────────────────

section('SOCIAL LINKS  (leave blank to skip)');

const githubUrl = await ask('GitHub profile URL');
const linkedinUrl = await ask('LinkedIn profile URL');
const twitterUrl = await ask('Twitter/X profile URL');
const mastodonUrl = await ask('Mastodon profile URL');
const emailAddress = await ask('Email address (e.g. you@example.com)');

// ─── Features ─────────────────────────────────────────────────────────────────

section('FEATURES');

const featureSearch = await askYN('Enable search?', true);
const featureRss = await askYN('Enable RSS feed?', true);
const featureMermaid = await askYN('Enable Mermaid diagrams in posts?', true);
const featureOg = await askYN('Enable Open Graph image generation?', true);
const featureProjects = await askYN('Enable /projects page?', true);
const featureBooks = await askYN('Enable /books page?', true);

const siteHostname = new URL(siteUrl).hostname;

const featureViewCounter = await askYN('Enable view counter?', false);
let viewCounterUrlProd = '';
let viewCounterUrlDev = '';
if (featureViewCounter) {
	viewCounterUrlProd = await ask('  View counter WebSocket URL (prod)', `wss://counter.${siteHostname}`);
	viewCounterUrlDev = await ask('  View counter WebSocket URL (dev)', 'ws://localhost:8000');
}

const featureContact = await askYN('Enable contact form?', false);
let contactUrlProd = '';
let contactUrlDev = '';
let turnstileSiteKey = '';
let turnstileSiteKeyDev = '';
if (featureContact) {
	contactUrlProd = await ask('  Contact API URL (prod)', `https://contact.${siteHostname}/api/v1/contact/`);
	contactUrlDev = await ask('  Contact API URL (dev)', 'http://localhost:8001/api/v1/contact/');
	turnstileSiteKey = await ask('  Cloudflare Turnstile site key (prod)');
	turnstileSiteKeyDev = await ask('  Cloudflare Turnstile site key (dev, use 1x00000000000000000000AA for local testing)', '1x00000000000000000000AA');
}

// ─── Theme ───────────────────────────────────────────────────────────────────

section('THEME');

console.log('Available themes: amber, emerald, indigo, rose\n');
let defaultTheme = await ask('Default theme', 'rose');
if (!['amber', 'emerald', 'indigo', 'rose'].includes(defaultTheme)) {
	console.log('  Unknown theme, falling back to rose.');
	defaultTheme = 'rose';
}

// ─── Homepage copy ────────────────────────────────────────────────────────────

section('HOMEPAGE');

const heroHeadline = await ask('Hero headline (your name in caps)', authorName.toUpperCase());
const heroRoleTitle = await ask('Current role title', jobTitle);
const heroRoleOrg = await ask('Current organisation name');
const heroRoleOrgUrl = await ask('Current organisation URL');
const heroRoleSummary = await ask('One-line role summary', `Building things at ${heroRoleOrg || 'my company'}.`);
const heroBody = await ask(
	'Homepage body text (single paragraph, use \\n for line breaks)',
	`I'm a ${jobTitle.toLowerCase()} writing about software.`
);

rl.close();

// ─── Build outputs ────────────────────────────────────────────────────────────

console.log('\n\nApplying configuration...\n');

// Helpers
const bool = (v) => (v ? 'true' : 'false');

// Social links array
const socialLinks = [];
if (githubUrl) socialLinks.push({ kind: 'github', label: 'GitHub', href: githubUrl });
if (linkedinUrl) socialLinks.push({ kind: 'linkedin', label: 'LinkedIn', href: linkedinUrl });
if (twitterUrl) socialLinks.push({ kind: 'twitter', label: 'Twitter', href: twitterUrl });
if (mastodonUrl) socialLinks.push({ kind: 'mastodon', label: 'Mastodon', href: mastodonUrl });
if (emailAddress) socialLinks.push({ kind: 'email', label: 'Email', href: `mailto:${emailAddress}` });

const socialTs = socialLinks
	.map(
		(s) =>
			`\t\t\t{\n\t\t\t\tkind: '${s.kind}',\n\t\t\t\tlabel: '${s.label}',\n\t\t\t\thref: '${s.href}'\n\t\t\t}`
	)
	.join(',\n');

// Navigation — always include HOME + BLOG; conditionally add PROJECTS and BOOKS
const navItems = [
	{ label: 'HOME', href: '/' },
	{ label: 'BLOG', href: '/posts' },
	...(featureProjects ? [{ label: 'PROJECTS', href: '/projects' }] : []),
	...(featureBooks ? [{ label: 'BOOKS', href: '/books' }] : [])
];
const navTs = navItems
	.map((n) => `\t\t\t{ label: '${n.label}', href: '${n.href}' }`)
	.join(',\n');

const tldLine = brandTld ? `\n\t\t\ttld: '${brandTld}',` : '';

// ── blog.config.ts ───────────────────────────────────────────────────────────

const blogConfig = `import type { SiteConfig } from './src/config/site';
import type { ThemeConfigInput } from './src/config/theme';

export interface BlogConfig {
\tsite: SiteConfig;
\ttheme: ThemeConfigInput;
}

/**
 * Customization entrypoint for the blog template.
 */
export const blogConfig: BlogConfig = {
\tsite: {
\t\turl: '${siteUrl}',
\t\tbrand: {
\t\t\tname: '${brandName}',${tldLine}
\t\t\taccentDot: ${accentDot}
\t\t},
\t\tmeta: {
\t\t\ttitle: '${pageTitle}',
\t\t\tdescription: '${metaDescription.replace(/'/g, "\\'")}',
\t\t\tlocale: 'en'
\t\t},
\t\tauthor: {
\t\t\tname: '${authorName}',
\t\t\tjobTitle: '${jobTitle}',
\t\t\tbio: '${authorBio.replace(/'/g, "\\'")}'
\t\t},
\t\tsocial: [
${socialTs}
\t\t],
\t\tnav: [
${navTs}
\t\t],
\t\togTagline: '${ogTagline}'
\t},
\ttheme: {
\t\tdefault: '${defaultTheme}'
\t}
};
`;

writeFileSync(join(root, 'blog.config.ts'), blogConfig);
console.log('  ✓  blog.config.ts');

// ── wrangler.jsonc ───────────────────────────────────────────────────────────

const makeVars = (isProd) => ({
	PUBLIC_FEATURE_SEARCH: bool(featureSearch),
	PUBLIC_FEATURE_VIEW_COUNTER: bool(featureViewCounter),
	PUBLIC_FEATURE_CONTACT: bool(featureContact),
	PUBLIC_FEATURE_OG: bool(featureOg),
	PUBLIC_FEATURE_MERMAID: bool(featureMermaid),
	PUBLIC_FEATURE_RSS: bool(featureRss),
	PUBLIC_FEATURE_BOOKS: bool(featureBooks),
	PUBLIC_FEATURE_PROJECTS: bool(featureProjects),
	...(featureViewCounter
		? { PUBLIC_VIEW_COUNTER_URL: isProd ? viewCounterUrlProd : viewCounterUrlDev }
		: {}),
	...(featureContact
		? {
				PUBLIC_CONTACT_URL: isProd ? contactUrlProd : contactUrlDev,
				PUBLIC_TURNSTILE_SITE_KEY: isProd ? turnstileSiteKey : turnstileSiteKeyDev
			}
		: {})
});

const wranglerObj = {
	$schema: '../node_modules/wrangler/config-schema.json',
	name: brandName.toLowerCase().replace(/[^a-z0-9-]/g, '-'),
	compatibility_date: new Date().toISOString().slice(0, 10),
	observability: { enabled: true },
	route: { pattern: siteHostname, custom_domain: true },
	vars: makeVars(true),
	main: 'dist/_worker.js/index.js',
	compatibility_flags: ['global_fetch_strictly_public', 'nodejs_compat'],
	assets: { binding: 'ASSETS', directory: 'dist' },
	env: { dev: { vars: makeVars(false) } }
};

writeFileSync(join(root, 'wrangler.jsonc'), JSON.stringify(wranglerObj, null, 2));
console.log('  ✓  wrangler.jsonc');

// ── home.md ──────────────────────────────────────────────────────────────────

const bodyText = heroBody.replace(/\\n/g, '\n\n');

const homeMd = `---
headline: ${heroHeadline}
currentRole:
  title: ${heroRoleTitle}
  org: ${heroRoleOrg}
  orgUrl: ${heroRoleOrgUrl}
  summary: ${heroRoleSummary}
---

${bodyText}
`;

writeFileSync(join(root, 'src/content/pages/home.md'), homeMd);
console.log('  ✓  src/content/pages/home.md');

// ─── Next steps ───────────────────────────────────────────────────────────────

console.log(`
╔══════════════════════════════════════════════════════════╗
║                  Setup complete!                        ║
╚══════════════════════════════════════════════════════════╝

Here's where to customise each part of the site:

  HOMEPAGE COPY
  └─ web/src/content/pages/home.md
     Edit the headline, role summary, and body text.

  SITE & AUTHOR
  └─ web/blog.config.ts
     Change brand name, meta description, social links,
     navigation, OG tagline, and default theme.

  BLOG POSTS
  └─ web/src/content/posts/
     Add Markdown/MDX files here. Delete the example posts.

${featureProjects ? `  PROJECTS
  └─ web/src/features/projects/content/projects.json
     Add your project entries.

` : ''}${featureBooks ? `  BOOKS
  └─ web/src/features/books/content/books.json
     Run \`npm run books:import\` to import a Goodreads export,
     or edit the JSON file directly.

` : ''}  FEATURE FLAGS
  └─ web/wrangler.jsonc  (vars section)
     Toggle PUBLIC_FEATURE_* values to enable/disable features.

  STYLES & THEME
  └─ web/src/styles/global.css
     Override global styles here.

Next steps:
  1. cd web && npm install
  2. npm run dev
  3. Open http://localhost:4321
`);
