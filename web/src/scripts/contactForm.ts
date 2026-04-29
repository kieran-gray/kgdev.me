import { generateAsciiArtText } from '@/lib/render/asciiArt';

function initContactForm() {
	const form = document.getElementById('contact-form') as HTMLFormElement | null;
	const overlay = document.getElementById('form-overlay') as HTMLDivElement | null;
	const statusAscii = document.getElementById('status-ascii') as HTMLPreElement | null;
	const submitBtn = document.getElementById('submit-btn') as HTMLButtonElement | null;

	let turnstileToken = '';

	if (!form || !overlay || !statusAscii || !submitBtn) return;

	const endpoint = form.dataset.endpoint ?? '';
	const sitekey = form.dataset.turnstileSitekey ?? '';

	const turnstileContainer = document.getElementById('turnstile-container');
	const turnstile = (window as any).turnstile;

	if (turnstileContainer && turnstile && sitekey) {
		turnstile.render('#turnstile-container', {
			sitekey,
			callback: (token: string) => {
				turnstileToken = token;
			},
			appearance: 'interaction-only'
		});
	}

	async function animateText(text: string) {
		statusAscii!.textContent = '';

		for (let i = 1; i <= text.length; i++) {
			const partial = text.substring(0, i);
			statusAscii!.textContent = generateAsciiArtText(partial, { spacing: 1 });
			await new Promise((r) => setTimeout(r, 100));
		}
	}

	form.addEventListener('submit', async (e) => {
		e.preventDefault();

		if (!turnstileToken && !window.location.hostname.includes('localhost')) {
			alert('Please complete the Turnstile challenge.');
			return;
		}

		const formData = new FormData(form);

		const name = formData.get('name');
		const email = formData.get('email');
		const message = formData.get('message');

		overlay.classList.remove('opacity-0', 'pointer-events-none');
		submitBtn.disabled = true;

		const animationPromise = animateText('SENDING...');
		const minimumDelayPromise = new Promise((r) => setTimeout(r, 1000));

		const apiPromise = fetch(endpoint, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ name, email, message, token: turnstileToken })
		}).then((res) => res.json());

		try {
			const [, response] = await Promise.all([
				Promise.all([animationPromise, minimumDelayPromise]),
				apiPromise
			]);

			if (response.success) {
				statusAscii.textContent = generateAsciiArtText('SENT!', { spacing: 1 });
				form.reset();

				if (turnstile) {
					turnstile.reset();
					turnstileToken = '';
				}
			} else {
				statusAscii.textContent = generateAsciiArtText('ERROR!', { spacing: 1 });
			}

			setTimeout(() => {
				overlay.classList.add('opacity-0', 'pointer-events-none');
				submitBtn.disabled = false;
			}, 3000);
		} catch (error) {
			console.error(error);

			await animationPromise;

			statusAscii.textContent = generateAsciiArtText('ERROR!', { spacing: 1 });

			setTimeout(() => {
				overlay.classList.add('opacity-0', 'pointer-events-none');
				submitBtn.disabled = false;
			}, 3000);
		}
	});
}

document.addEventListener('astro:page-load', initContactForm);
