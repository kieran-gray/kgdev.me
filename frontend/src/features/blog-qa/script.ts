export {};

interface Source {
	chunk_id: number;
	heading: string;
	score: number;
}

interface Reference {
	title: string;
	url: string;
}

interface MetaPayload {
	sources: Source[];
	references?: Reference[];
	cached: boolean;
	model: string;
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
	const referencesWrap = root.querySelector<HTMLDivElement>('#blog-qa-references-wrap');
	const referencesList = root.querySelector<HTMLUListElement>('#blog-qa-references');

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
		!sourcesList ||
		!referencesWrap ||
		!referencesList
	) {
		return;
	}

	document.body.appendChild(dialog);

	let abortController: AbortController | null = null;

	function setBusy(busy: boolean) {
		submitBtn!.disabled = busy;
		submitLabel!.textContent = busy ? 'Thinking…' : 'Ask';
	}

	function abortInFlight() {
		abortController?.abort();
	}

	function showStatus(state: 'loading' | 'ok' | 'error', message?: string) {
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
		referencesList!.innerHTML = '';
		referencesWrap!.hidden = true;
	}

	function renderSources(sources: Source[]) {
		sourcesList!.innerHTML = '';
		if (sources.length === 0) {
			sourcesWrap!.hidden = true;
			return;
		}
		for (const s of sources) {
			const li = document.createElement('li');
			const heading = s.heading || '(intro)';
			li.textContent = `${heading} — ${(s.score * 100).toFixed(0)}%`;
			sourcesList!.appendChild(li);
		}
		sourcesWrap!.hidden = false;
	}

	function renderReferences(references: Reference[]) {
		referencesList!.innerHTML = '';
		if (references.length === 0) {
			referencesWrap!.hidden = true;
			return;
		}
		for (const r of references) {
			const li = document.createElement('li');
			const a = document.createElement('a');
			a.href = r.url;
			a.target = '_blank';
			a.rel = 'noopener noreferrer';
			a.textContent = r.title;
			li.appendChild(a);
			referencesList!.appendChild(li);
		}
		referencesWrap!.hidden = false;
	}

	function appendDelta(text: string) {
		answerEl!.textContent = (answerEl!.textContent ?? '') + text;
	}

	function openDialog() {
		clearResult();
		dialog!.showModal();
		input!.focus();
	}

	function closeDialog() {
		abortInFlight();
		if (dialog!.open) dialog!.close();
	}

	function parseSseBlock(block: string): { event: string; data: string } | null {
		const lines = block.split('\n');
		let event = 'message';
		const dataLines: string[] = [];
		for (const line of lines) {
			if (line.startsWith('event:')) {
				event = line.slice(6).trim();
			} else if (line.startsWith('data:')) {
				dataLines.push(line.slice(5).trim());
			}
		}
		if (dataLines.length === 0) return null;
		return { event, data: dataLines.join('\n') };
	}

	async function consumeSseStream(body: ReadableStream<Uint8Array>): Promise<void> {
		const reader = body.getReader();
		const decoder = new TextDecoder();
		let buffer = '';
		let sawDone = false;

		const handleBlock = (block: string) => {
			const parsed = parseSseBlock(block);
			if (!parsed) return;

			if (parsed.event === 'meta') {
				const meta = JSON.parse(parsed.data) as MetaPayload;
				showStatus('ok', meta.cached ? 'Cached answer' : 'Drafting answer…');
				renderSources(meta.sources);
				renderReferences(meta.references ?? []);
			} else if (parsed.event === 'delta') {
				const payload = JSON.parse(parsed.data) as { text: string };
				appendDelta(payload.text);
			} else if (parsed.event === 'done') {
				sawDone = true;
				showStatus(
					'ok',
					statusEl!.dataset.state === 'error' ? (statusEl!.textContent ?? '') : 'Done'
				);
			} else if (parsed.event === 'error') {
				const payload = JSON.parse(parsed.data) as { message: string };
				showStatus('error', payload.message);
			}
		};

		const flushFramedBlocks = () => {
			while (true) {
				const idx = buffer.indexOf('\n\n');
				if (idx === -1) break;
				const block = buffer.slice(0, idx);
				buffer = buffer.slice(idx + 2);
				handleBlock(block);
			}
		};

		while (true) {
			const { done, value } = await reader.read();
			if (done) break;
			buffer += decoder.decode(value, { stream: true }).replace(/\r\n?/g, '\n');
			flushFramedBlocks();
		}

		buffer += decoder.decode().replace(/\r\n?/g, '\n');
		flushFramedBlocks();
		const trailing = buffer.replace(/^\n+|\n+$/g, '');
		if (trailing.length > 0) {
			handleBlock(trailing);
		}

		if (!sawDone && !answerEl!.textContent) {
			showStatus('error', 'Stream ended without a response.');
		}
	}

	async function submit(question: string) {
		clearResult();
		showStatus('loading', 'Searching the post and drafting an answer…');
		setBusy(true);
		const controller = new AbortController();
		abortController = controller;
		const { signal } = controller;

		try {
			const res = await fetch(`${endpoint}/${encodeURIComponent(slug!)}`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json', Accept: 'text/event-stream' },
				body: JSON.stringify({ question }),
				signal
			});

			const contentType = res.headers.get('content-type') ?? '';

			if (!res.ok || contentType.includes('application/json')) {
				let message = `Request failed (${res.status})`;
				try {
					const body = (await res.json()) as ErrorResponse;
					if ('error' in body) message = body.error;
				} catch {
					/* keep default */
				}
				showStatus('error', message);
				return;
			}

			if (!res.body) {
				showStatus('error', 'No response body.');
				return;
			}

			await consumeSseStream(res.body);
		} catch (err) {
			if (signal.aborted) return;
			showStatus('error', err instanceof Error ? err.message : 'Network error');
		} finally {
			if (abortController === controller) {
				abortController = null;
				setBusy(false);
			}
		}
	}

	const onOpen = () => openDialog();
	const onClose = () => closeDialog();
	const onCancel = () => {
		abortInFlight();
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
		abortInFlight();
		openBtn.removeEventListener('click', onOpen);
		closeBtn.removeEventListener('click', onClose);
		dialog.removeEventListener('cancel', onCancel);
		dialog.removeEventListener('click', onBackdropClick);
		input.removeEventListener('input', onInput);
		form.removeEventListener('submit', onSubmit);
		if (dialog.open) dialog.close();
		root.appendChild(dialog);
	};
}

document.addEventListener('astro:before-preparation', teardown);
document.addEventListener('astro:page-load', setup);
