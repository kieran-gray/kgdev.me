export {};

interface Source {
	chunk_id: number;
	heading: string;
	score: number;
}

interface AnswerResponse {
	answer: string;
	sources: Source[];
	model: string;
	cached: boolean;
}

interface ErrorResponse {
	success: false;
	error: string;
}

let cleanup: (() => void) | null = null;

function teardown() {
	if (cleanup) {
		cleanup();
		cleanup = null;
	}
}

function setup() {
	teardown();

	const root = document.getElementById('blog-qa') as HTMLElement | null;
	if (!root) return;

	const slug = root.dataset.slug;
	const endpoint = root.dataset.endpoint;
	if (!slug || !endpoint) return;

	const openBtn = root.querySelector<HTMLButtonElement>('#blog-qa-open');
	const dialog = root.querySelector<HTMLDialogElement>('#blog-qa-dialog');
	const closeBtn = root.querySelector<HTMLButtonElement>('#blog-qa-close');
	const form = root.querySelector<HTMLFormElement>('#blog-qa-form');
	const input = root.querySelector<HTMLTextAreaElement>('#blog-qa-input');
	const submitBtn = root.querySelector<HTMLButtonElement>('#blog-qa-submit');
	const submitLabel = root.querySelector<HTMLSpanElement>('.blog-qa-submit-label');
	const charCount = root.querySelector<HTMLSpanElement>('#blog-qa-charcount');
	const resultBox = root.querySelector<HTMLDivElement>('#blog-qa-result');
	const statusEl = root.querySelector<HTMLDivElement>('#blog-qa-status');
	const answerEl = root.querySelector<HTMLDivElement>('#blog-qa-answer');
	const sourcesWrap = root.querySelector<HTMLDetailsElement>('#blog-qa-sources-wrap');
	const sourcesList = root.querySelector<HTMLUListElement>('#blog-qa-sources');

	if (
		!openBtn ||
		!dialog ||
		!closeBtn ||
		!form ||
		!input ||
		!submitBtn ||
		!submitLabel ||
		!charCount ||
		!resultBox ||
		!statusEl ||
		!answerEl ||
		!sourcesWrap ||
		!sourcesList
	) {
		return;
	}

	let inFlight = false;

	function setBusy(busy: boolean) {
		inFlight = busy;
		submitBtn!.disabled = busy;
		submitLabel!.textContent = busy ? 'Thinking…' : 'Ask';
	}

	function showResult(state: 'loading' | 'ok' | 'error', message?: string) {
		resultBox!.hidden = false;
		statusEl!.textContent = message ?? '';
		statusEl!.dataset.state = state;
	}

	function clearResult() {
		resultBox!.hidden = true;
		statusEl!.textContent = '';
		statusEl!.removeAttribute('data-state');
		answerEl!.textContent = '';
		sourcesList!.innerHTML = '';
		sourcesWrap!.hidden = true;
	}

	function renderAnswer(data: AnswerResponse) {
		answerEl!.textContent = data.answer;
		sourcesList!.innerHTML = '';
		if (data.sources.length === 0) {
			sourcesWrap!.hidden = true;
			return;
		}
		for (const s of data.sources) {
			const li = document.createElement('li');
			const heading = s.heading || '(intro)';
			li.textContent = `${heading} — ${(s.score * 100).toFixed(0)}%`;
			sourcesList!.appendChild(li);
		}
		sourcesWrap!.hidden = false;
	}

	function openDialog() {
		clearResult();
		dialog!.showModal();
		input!.focus();
	}

	function closeDialog() {
		if (inFlight) return;
		dialog!.close();
	}

	async function submit(question: string) {
		clearResult();
		showResult('loading', 'Searching the post and drafting an answer…');
		setBusy(true);

		try {
			const res = await fetch(`${endpoint}/${encodeURIComponent(slug!)}`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ question })
			});

			const text = await res.text();
			let body: AnswerResponse | ErrorResponse | null = null;
			try {
				body = text ? JSON.parse(text) : null;
			} catch {
				/* fall through */
			}

			if (!res.ok) {
				const msg = body && 'error' in body ? body.error : `Request failed (${res.status})`;
				showResult('error', msg);
				return;
			}

			if (!body || !('answer' in body)) {
				showResult('error', 'Malformed response from the server.');
				return;
			}

			const data = body as AnswerResponse;
			showResult('ok', data.cached ? 'Cached answer' : 'Fresh answer');
			renderAnswer(data);
		} catch (err) {
			showResult('error', err instanceof Error ? err.message : 'Network error');
		} finally {
			setBusy(false);
		}
	}

	const onOpen = () => openDialog();
	const onClose = () => closeDialog();
	const onCancel = (e: Event) => {
		if (inFlight) e.preventDefault();
	};
	const onBackdropClick = (e: MouseEvent) => {
		if (e.target === dialog) closeDialog();
	};
	const onInput = () => {
		const len = input.value.length;
		charCount.textContent = `${len} / 500`;
	};
	const onSubmit = (e: Event) => {
		e.preventDefault();
		const value = input.value.trim();
		if (value.length === 0) return;
		void submit(value);
	};

	openBtn.addEventListener('click', onOpen);
	closeBtn.addEventListener('click', onClose);
	dialog.addEventListener('cancel', onCancel);
	dialog.addEventListener('click', onBackdropClick);
	input.addEventListener('input', onInput);
	form.addEventListener('submit', onSubmit);
	onInput();

	cleanup = () => {
		openBtn.removeEventListener('click', onOpen);
		closeBtn.removeEventListener('click', onClose);
		dialog.removeEventListener('cancel', onCancel);
		dialog.removeEventListener('click', onBackdropClick);
		input.removeEventListener('input', onInput);
		form.removeEventListener('submit', onSubmit);
		if (dialog.open) dialog.close();
	};
}

document.addEventListener('astro:before-preparation', teardown);
document.addEventListener('astro:page-load', setup);
