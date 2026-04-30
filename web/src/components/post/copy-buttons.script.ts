export {};

function addCopyButtons() {
	const blocks = document.querySelectorAll<HTMLPreElement>('.content pre:not(.mermaid)');
	for (const pre of blocks) {
		if (pre.querySelector('.copy-btn')) continue;
		const code = pre.querySelector('code');
		if (!code) continue;

		const btn = document.createElement('button');
		btn.className = 'copy-btn';
		btn.textContent = 'copy';
		btn.setAttribute('aria-label', 'Copy code to clipboard');

		btn.addEventListener('click', async () => {
			try {
				await navigator.clipboard.writeText(code.innerText);
				btn.textContent = 'copied!';
				btn.classList.add('copied');
				setTimeout(() => {
					btn.textContent = 'copy';
					btn.classList.remove('copied');
				}, 2000);
			} catch {
				btn.textContent = 'error';
				setTimeout(() => {
					btn.textContent = 'copy';
				}, 2000);
			}
		});

		pre.appendChild(btn);
	}
}

document.addEventListener('astro:page-load', addCopyButtons);
