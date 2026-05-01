export {};

function setupToc() {
	const links = document.querySelectorAll<HTMLAnchorElement>('.toc-link[data-slug]');
	if (!links.length) return;

	const slugToLink = new Map<string, HTMLAnchorElement>();
	links.forEach((l) => slugToLink.set(l.dataset.slug!, l));

	const contentHeadings = Array.from(document.querySelectorAll<HTMLElement>('.post-main h1[id], .content h2[id], .content h3[id]'));
	if (!contentHeadings.length) return;

	let activeSlug: string | null = null;

	function activate(slug: string | null) {
		if (slug === activeSlug) return;
		links.forEach((l) => l.classList.remove('toc-active'));
		if (slug) slugToLink.get(slug)?.classList.add('toc-active');
		activeSlug = slug;
	}

	activate(contentHeadings[0]?.id ?? null);

	const observer = new IntersectionObserver(
		(entries) => {
			for (const entry of entries) {
				if (entry.isIntersecting) activate(entry.target.id);
			}
		},
		{ rootMargin: '0px 0px -80% 0px' }
	);

	contentHeadings.forEach((h) => {
		if (h.id) observer.observe(h);
	});
}

function setupToggle() {
	const btn = document.querySelector<HTMLButtonElement>('.toc-toggle');
	const list = document.querySelector<HTMLOListElement>('.toc-list');
	if (!btn || !list) return;

	btn.addEventListener('click', () => {
		const collapsed = btn.getAttribute('aria-expanded') === 'false';
		btn.setAttribute('aria-expanded', collapsed ? 'true' : 'false');
		list.classList.toggle('toc-list-hidden', !collapsed);
	});
}

document.addEventListener('astro:page-load', () => {
	setupToc();
	setupToggle();
});
