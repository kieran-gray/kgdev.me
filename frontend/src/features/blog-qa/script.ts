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

interface SseFrame {
	event: string;
	data: string;
}

interface Elements {
	root: HTMLElement;
	openBtn: HTMLButtonElement;
	dialog: HTMLDialogElement;
	closeBtn: HTMLButtonElement;
	form: HTMLFormElement;
	input: HTMLTextAreaElement;
	submitBtn: HTMLButtonElement;
	submitLabel: HTMLSpanElement;
	charCount: HTMLSpanElement;
	resultBox: HTMLDivElement;
	statusEl: HTMLDivElement;
	answerEl: HTMLDivElement;
	sourcesWrap: HTMLDetailsElement;
	sourcesList: HTMLUListElement;
	referencesWrap: HTMLDivElement;
	referencesList: HTMLUListElement;
}

const selectors = {
	openBtn: '#blog-qa-open',
	dialog: '#blog-qa-dialog',
	closeBtn: '#blog-qa-close',
	form: '#blog-qa-form',
	input: '#blog-qa-input',
	submitBtn: '#blog-qa-submit',
	submitLabel: '.blog-qa-submit-label',
	charCount: '#blog-qa-charcount',
	resultBox: '#blog-qa-result',
	statusEl: '#blog-qa-status',
	answerEl: '#blog-qa-answer',
	sourcesWrap: '#blog-qa-sources-wrap',
	sourcesList: '#blog-qa-sources',
	referencesWrap: '#blog-qa-references-wrap',
	referencesList: '#blog-qa-references'
} as const;

function findElements(root: HTMLElement): Elements | null {
	const result: any = { root };

	for (const [key, sel] of Object.entries(selectors)) {
		const el = root.querySelector(sel);
		if (!el) return null;
		result[key] = el;
	}

	return result as Elements;
}

function parseSseFrame(block: string): SseFrame | null {
	let event = 'message';
	const dataLines: string[] = [];
	for (const line of block.split('\n')) {
		if (line.startsWith('event:')) event = line.slice(6).trim();
		else if (line.startsWith('data:')) dataLines.push(line.slice(5).trim());
	}
	if (dataLines.length === 0) return null;
	return { event, data: dataLines.join('\n') };
}

async function* readSseFrames(stream: ReadableStream<Uint8Array>): AsyncGenerator<SseFrame> {
	const reader = stream.getReader();
	const decoder = new TextDecoder();
	let buffer = '';

	function* drainComplete(): Generator<SseFrame> {
		while (true) {
			const idx = buffer.indexOf('\n\n');
			if (idx === -1) break;
			const block = buffer.slice(0, idx);
			buffer = buffer.slice(idx + 2);
			const frame = parseSseFrame(block);
			if (frame) yield frame;
		}
	}

	while (true) {
		const { done, value } = await reader.read();
		if (done) break;
		buffer += decoder.decode(value, { stream: true }).replace(/\r\n?/g, '\n');
		yield* drainComplete();
	}
	buffer += decoder.decode().replace(/\r\n?/g, '\n');
	yield* drainComplete();

	const tail = buffer.replace(/^\n+|\n+$/g, '');
	if (tail.length > 0) {
		const frame = parseSseFrame(tail);
		if (frame) yield frame;
	}
}

let cleanup: (() => void) | null = null;

function teardown() {
	cleanup?.();
	cleanup = null;
}

function setup() {
	teardown();

	const root = document.getElementById('blog-qa');
	if (!root) return;

	const { slug, endpoint } = root.dataset;
	if (!slug || !endpoint) return;

	const found = findElements(root);
	if (!found) return;
	const els: Elements = found;

	document.body.appendChild(els.dialog);

	const disposers: Array<() => void> = [];
	const on = <T extends EventTarget>(target: T, event: string, handler: (e: any) => void) => {
		target.addEventListener(event, handler);
		disposers.push(() => target.removeEventListener(event, handler));
	};

	let abortController: AbortController | null = null;
	let removeViewportListeners: (() => void) | null = null;

	function setViewportVars() {
		const viewport = window.visualViewport;
		const height = viewport?.height ?? window.innerHeight;
		const width = viewport?.width ?? window.innerWidth;
		const top = (viewport?.offsetTop ?? 0) + height / 2;
		const left = (viewport?.offsetLeft ?? 0) + width / 2;
		els.dialog.style.setProperty('--blog-qa-viewport-top', `${top}px`);
		els.dialog.style.setProperty('--blog-qa-viewport-left', `${left}px`);
		els.dialog.style.setProperty('--blog-qa-viewport-width', `${width}px`);
		els.dialog.style.setProperty('--blog-qa-viewport-height', `${height}px`);
	}

	function startViewportTracking() {
		stopViewportTracking();
		setViewportVars();
		const viewport = window.visualViewport;
		if (!viewport) return;
		const onChange = () => setViewportVars();
		viewport.addEventListener('resize', onChange);
		viewport.addEventListener('scroll', onChange);
		removeViewportListeners = () => {
			viewport.removeEventListener('resize', onChange);
			viewport.removeEventListener('scroll', onChange);
		};
	}

	function stopViewportTracking() {
		removeViewportListeners?.();
		removeViewportListeners = null;
		els.dialog.style.removeProperty('--blog-qa-viewport-top');
		els.dialog.style.removeProperty('--blog-qa-viewport-left');
		els.dialog.style.removeProperty('--blog-qa-viewport-width');
		els.dialog.style.removeProperty('--blog-qa-viewport-height');
	}

	function setBusy(busy: boolean) {
		els.submitBtn.disabled = busy;
		els.submitLabel.textContent = busy ? 'Thinking…' : 'Ask';
	}

	function showStatus(state: 'loading' | 'ok' | 'error', message?: string) {
		els.resultBox.hidden = false;
		els.statusEl.textContent = message ?? '';
		els.statusEl.dataset.state = state;
	}

	function clearResult() {
		els.resultBox.hidden = true;
		els.statusEl.textContent = '';
		els.statusEl.removeAttribute('data-state');
		els.answerEl.textContent = '';
		els.sourcesList.innerHTML = '';
		els.sourcesWrap.hidden = true;
		els.referencesList.innerHTML = '';
		els.referencesWrap.hidden = true;
	}

	function renderSources(sources: Source[]) {
		els.sourcesList.innerHTML = '';
		if (sources.length === 0) {
			els.sourcesWrap.hidden = true;
			return;
		}
		for (const s of sources) {
			const li = document.createElement('li');
			const heading = s.heading || '(intro)';
			li.textContent = `${heading} — ${(s.score * 100).toFixed(0)}%`;
			els.sourcesList.appendChild(li);
		}
		els.sourcesWrap.hidden = false;
	}

	function renderReferences(references: Reference[]) {
		els.referencesList.innerHTML = '';
		if (references.length === 0) {
			els.referencesWrap.hidden = true;
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
			els.referencesList.appendChild(li);
		}
		els.referencesWrap.hidden = false;
	}

	function handleFrame(frame: SseFrame): { done: boolean } {
		switch (frame.event) {
			case 'meta': {
				const meta = JSON.parse(frame.data) as MetaPayload;
				showStatus('ok', meta.cached ? 'Cached answer' : 'Drafting answer…');
				renderSources(meta.sources);
				renderReferences(meta.references ?? []);
				return { done: false };
			}
			case 'delta': {
				const { text } = JSON.parse(frame.data) as { text: string };
				els.answerEl.textContent = (els.answerEl.textContent ?? '') + text;
				return { done: false };
			}
			case 'done': {
				if (els.statusEl.dataset.state !== 'error') showStatus('ok', 'Done');
				return { done: true };
			}
			case 'error': {
				const { message } = JSON.parse(frame.data) as { message: string };
				showStatus('error', message);
				return { done: false };
			}
			default:
				return { done: false };
		}
	}

	async function consumeStream(body: ReadableStream<Uint8Array>) {
		let sawDone = false;
		for await (const frame of readSseFrames(body)) {
			if (handleFrame(frame).done) sawDone = true;
		}
		if (!sawDone && !els.answerEl.textContent) {
			showStatus('error', 'Stream ended without a response.');
		}
	}

	function abortInFlight() {
		abortController?.abort();
	}

	function openDialog() {
		clearResult();
		els.dialog.showModal();
		startViewportTracking();
		els.input.focus();
	}

	function closeDialog() {
		abortInFlight();
		stopViewportTracking();
		if (els.dialog.open) els.dialog.close();
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

			await consumeStream(res.body);
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

	on(els.openBtn, 'click', () => openDialog());
	on(els.closeBtn, 'click', () => closeDialog());
	on(els.dialog, 'cancel', () => abortInFlight());
	on(els.dialog, 'click', (e: MouseEvent) => {
		if (e.target === els.dialog) closeDialog();
	});
	on(els.input, 'input', () => {
		els.charCount.textContent = `${els.input.value.length} / 500`;
	});
	on(els.form, 'submit', (e: Event) => {
		e.preventDefault();
		const value = els.input.value.trim();
		if (value.length === 0) return;
		void submit(value);
	});

	els.charCount.textContent = `${els.input.value.length} / 500`;

	cleanup = () => {
		abortInFlight();
		stopViewportTracking();
		for (const dispose of disposers) dispose();
		if (els.dialog.open) els.dialog.close();
		els.root.appendChild(els.dialog);
	};
}

document.addEventListener('astro:before-preparation', teardown);
document.addEventListener('astro:page-load', setup);
