export {};
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let pf: any = null;
let selectedIndex = -1;

async function getPagefind() {
	if (pf) return pf;
	try {
		const pagefindUrl = '/pagefind/' + 'pagefind.js';
		pf = await import(/* @vite-ignore */ pagefindUrl);
		await pf.init();
	} catch {
		pf = null;
	}
	return pf;
}

const dialog = () => document.getElementById('search-dialog') as HTMLDialogElement | null;
const input = () => document.getElementById('search-input') as HTMLInputElement | null;
const hint = () => document.getElementById('search-hint') as HTMLParagraphElement | null;
const resultsEl = () => document.getElementById('search-results') as HTMLDivElement | null;
const searchTargets = () => dialog()?.dataset.searchTargets ?? 'posts';
const emptyHint = () => `Type to search ${searchTargets()}...`;

function openSearch() {
	const d = dialog();
	if (!d) return;
	d.showModal();
	const inp = input();
	if (!inp) return;
	inp.value = '';
	inp.focus();
	showHint(emptyHint());
	const r = resultsEl();
	if (r) r.innerHTML = '';
	selectedIndex = -1;
}

function closeSearch() {
	dialog()?.close();
	selectedIndex = -1;
}

function showHint(msg: string) {
	const h = hint();
	if (!h) return;
	h.textContent = msg;
	h.style.display = '';
}

function hideHint() {
	const h = hint();
	if (h) h.style.display = 'none';
}

function sectionLabel(url: string): string {
	const path = url.replace(/^https?:\/\/[^/]+/, '').replace(/\/$/, '');
	const seg = path.split('/').filter(Boolean)[0]?.toUpperCase();
	if (!seg) return 'HOME';
	if (seg === 'POSTS') return 'POST';
	if (seg === 'PROJECTS') return 'PROJECT';
	return seg;
}

async function runSearch(query: string) {
	const pagefind = await getPagefind();
	if (!pagefind) {
		showHint('Search unavailable — run a production build first.');
		return;
	}

	const search = await pagefind.search(query);
	const data = await Promise.all(
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		search.results.slice(0, 8).map((r: any) => r.data())
	);

	const r = resultsEl();
	if (!r) return;

	if (data.length === 0) {
		r.innerHTML = '';
		showHint(`No results for "${query}"`);
		selectedIndex = -1;
		return;
	}

	hideHint();
	selectedIndex = -1;

	r.innerHTML = data
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		.map(
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			(result: any, i: number) => `
      <a
        href="${result.url}"
        class="search-result"
        role="option"
        aria-selected="false"
        data-index="${i}"
      >
        <span class="search-result-section">${sectionLabel(result.url)}</span>
        <span class="search-result-title">${result.meta?.title ?? result.url}</span>
        <span class="search-result-excerpt">${result.excerpt}</span>
      </a>
    `
		)
		.join('');

	r.querySelectorAll<HTMLAnchorElement>('.search-result').forEach((el) => {
		el.addEventListener('mouseenter', () => {
			setSelected(Number(el.dataset.index));
		});
		el.addEventListener('click', closeSearch);
	});
}

function resultItems() {
	return Array.from(document.querySelectorAll<HTMLAnchorElement>('.search-result'));
}

function setSelected(i: number) {
	resultItems().forEach((el, idx) => {
		el.setAttribute('aria-selected', idx === i ? 'true' : 'false');
	});
	selectedIndex = i;
	resultItems()[i]?.scrollIntoView({ block: 'nearest' });
}

function handleInputKey(e: KeyboardEvent) {
	const items = resultItems();
	if (e.key === 'ArrowDown') {
		e.preventDefault();
		setSelected(Math.min(selectedIndex + 1, items.length - 1));
	} else if (e.key === 'ArrowUp') {
		e.preventDefault();
		setSelected(Math.max(selectedIndex - 1, 0));
	} else if (e.key === 'Enter') {
		e.preventDefault();
		const target = selectedIndex >= 0 ? items[selectedIndex] : items[0];
		target?.click();
	}
}

function setup() {
	const d = dialog();
	const inp = input();
	if (!d || !inp) return;

	d.addEventListener('click', (e) => {
		if (e.target === d) closeSearch();
	});

	let debounce: ReturnType<typeof setTimeout>;
	inp.addEventListener('input', () => {
		clearTimeout(debounce);
		const q = inp.value.trim();
		if (!q) {
			showHint(emptyHint());
			const r = resultsEl();
			if (r) r.innerHTML = '';
			selectedIndex = -1;
			return;
		}
		debounce = setTimeout(() => runSearch(q), 150);
	});

	inp.addEventListener('keydown', handleInputKey);
}

let wired = false;
function wireGlobal() {
	if (wired) return;
	wired = true;
	document.addEventListener('keydown', (e) => {
		if ((e.ctrlKey || e.metaKey) && e.key === 'p') {
			e.preventDefault();
			dialog()?.open ? closeSearch() : openSearch();
		}
	});
	document.addEventListener('search:open', openSearch);
}

wireGlobal();
setup();
document.addEventListener('astro:page-load', setup);
document.addEventListener('astro:before-preparation', closeSearch);
