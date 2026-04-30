export {};
let ws: WebSocket | null = null;

const teardown = () => {
	ws?.close();
	ws = null;
};

const setup = () => {
	teardown();

	const container = document.getElementById('view-counter');
	if (!container) return;

	const slug = container.dataset.slug;
	const wsBase = container.dataset.wsUrl;
	if (!slug || !wsBase) return;

	const liveEl = document.getElementById('vc-live');
	const totalEl = document.getElementById('vc-total');
	const sepEl = document.getElementById('vc-sep');

	ws = new WebSocket(`${wsBase}/api/v1/connect/${encodeURIComponent(slug)}`);

	ws.onmessage = (event) => {
		try {
			const { live, total } = JSON.parse(event.data as string) as {
				live: number;
				total: number;
			};

			if (liveEl) liveEl.textContent = `${live} reading now`;
			if (totalEl) totalEl.textContent = `${total} views`;
			if (sepEl) sepEl.classList.remove('hidden');
		} catch {
			/* ignore */
		}
	};

	ws.onerror = teardown;
	ws.onclose = teardown;
};

document.addEventListener('astro:before-preparation', teardown);
document.addEventListener('astro:page-load', setup);
