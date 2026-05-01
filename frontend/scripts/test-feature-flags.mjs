import { execSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { join } from 'node:path';

const root = process.cwd();
const dist = join(root, 'dist');
const astroCache = join(root, '.astro');
const viteCache = join(root, 'node_modules/.vite');

function runBuild(envOverrides) {
	// Yikes
	rmSync(dist, { recursive: true, force: true });
	rmSync(astroCache, { recursive: true, force: true });
	rmSync(viteCache, { recursive: true, force: true });
	execSync('npm run build', {
		stdio: 'inherit',
		env: { ...process.env, ...envOverrides }
	});
}

function read(path) {
	return readFileSync(join(dist, path), 'utf8');
}

function assert(condition, message, failures) {
	if (!condition) failures.push(message);
}

const scenarios = [
	{
		name: 'search',
		env: { PUBLIC_FEATURE_SEARCH: 'false' },
		check() {
			const failures = [];
			const html = read('index.html');
			assert(!existsSync(join(dist, 'pagefind')), 'Expected no dist/pagefind directory when search is disabled', failures);
			assert(!html.includes('data-pagefind-body'), 'Expected no data-pagefind-body attribute when search is disabled', failures);
			assert(!html.includes('search-trigger'), 'Expected no search trigger when search is disabled', failures);
			assert(!html.includes('search-dialog'), 'Expected no Search dialog markup when search is disabled', failures);
			return failures;
		}
	},
	{
		name: 'viewCounter',
		env: { PUBLIC_FEATURE_VIEW_COUNTER: 'false' },
		check() {
			const failures = [];
			const html = read('posts/blog-view-counter/index.html');
			assert(!html.includes('id="view-counter"'), 'Expected no ViewCounter mount when viewCounter is disabled', failures);
			assert(!html.includes('vc-live'), 'Expected no ViewCounter script payload when viewCounter is disabled', failures);
			return failures;
		}
	},
	{
		name: 'contact',
		env: { PUBLIC_FEATURE_CONTACT: 'false' },
		check() {
			const failures = [];
			const html = read('index.html');
			assert(!existsSync(join(dist, 'contact/index.html')), 'Expected no /contact route when contact is disabled', failures);
			assert(!html.includes('href="/contact"'), 'Expected no footer contact link when contact is disabled', failures);
			return failures;
		}
	},
	{
		name: 'og',
		env: { PUBLIC_FEATURE_OG: 'false' },
		check() {
			const failures = [];
			const html = read('posts/blog-view-counter/index.html');
			assert(!existsSync(join(dist, 'og/default.png')), 'Expected no /og/default.png when og is disabled', failures);
			assert(!existsSync(join(dist, 'og/blog-view-counter.png')), 'Expected no /og/[slug].png when og is disabled', failures);
			assert(!html.includes('/og/blog-view-counter.png'), 'Expected post page not to reference generated OG image when og is disabled', failures);
			return failures;
		}
	},
	{
		name: 'mermaid',
		env: { PUBLIC_FEATURE_MERMAID: 'false' },
		check() {
			const failures = [];
			const astroDir = join(dist, '_astro');
			const entries = existsSync(astroDir) ? readdirSync(astroDir) : [];
			assert(
				!entries.some((entry) => entry.includes('mermaid.core')),
				'Expected no Mermaid bundle in client output when mermaid is disabled',
				failures
			);
			return failures;
		}
	},
	{
		name: 'rss',
		env: { PUBLIC_FEATURE_RSS: 'false' },
		check() {
			const failures = [];
			const html = read('index.html');
			assert(!existsSync(join(dist, 'rss.xml')), 'Expected no /rss.xml route when rss is disabled', failures);
			assert(
				!html.includes('type="application/rss+xml"'),
				'Expected no RSS alternate link in head when rss is disabled',
				failures
			);
			return failures;
		}
	},
	{
		name: 'projects',
		env: { PUBLIC_FEATURE_PROJECTS: 'false' },
		check() {
			const failures = [];
			const homeHtml = read('index.html');
			const tagsHtml = read('tags/index.html');
			assert(!existsSync(join(dist, 'projects/index.html')), 'Expected no /projects route when projects is disabled', failures);
			assert(!homeHtml.includes('Pinned Projects') && !homeHtml.includes('Latest Projects'), 'Expected no projects section on the homepage when projects is disabled', failures);
			assert(!homeHtml.includes('href="/projects"'), 'Expected no homepage projects link when projects is disabled', failures);
			assert(!homeHtml.includes('>PROJECTS<'), 'Expected no projects nav item when projects is disabled', failures);
			assert(!tagsHtml.includes('projects'), 'Expected tags index not to advertise project counts when projects is disabled', failures);
			return failures;
		}
	},
	{
		name: 'books',
		env: { PUBLIC_FEATURE_BOOKS: 'false' },
		check() {
			const failures = [];
			const homeHtml = read('index.html');
			assert(!existsSync(join(dist, 'books/index.html')), 'Expected no /books route when books is disabled', failures);
			assert(!homeHtml.includes('>BOOKS<'), 'Expected no books nav item when books is disabled', failures);
			assert(!homeHtml.includes('href="/books"'), 'Expected no books link when books is disabled', failures);
			return failures;
		}
	},
	{
		name: 'blogQa',
		env: { PUBLIC_FEATURE_BLOG_QA: 'false' },
		check() {
			const failures = [];
			const html = read('posts/blog-view-counter/index.html');
			assert(!html.includes('id="blog-qa"'), 'Expected no Blog QA mount when blogQa is disabled', failures);
			return failures;
		}
	},
];

const scenarioFailures = [];

for (const scenario of scenarios) {
	console.log(`\n=== ${scenario.name} ===`);
	runBuild(scenario.env);
	const failures = scenario.check();
	if (failures.length === 0) {
		console.log(`PASS: ${scenario.name}`);
		continue;
	}

	console.log(`FAIL: ${scenario.name}`);
	for (const failure of failures) {
		console.log(`  - ${failure}`);
	}
	scenarioFailures.push({ name: scenario.name, failures });
}

if (scenarioFailures.length > 0) {
	console.log('\nFeature-flag matrix found failures:');
	for (const scenario of scenarioFailures) {
		console.log(`- ${scenario.name}`);
		for (const failure of scenario.failures) {
			console.log(`  - ${failure}`);
		}
	}
	process.exit(1);
}

console.log('\nAll feature-flag matrix scenarios passed.');
