// Runtime helpers for applying theme + scheme to the document.

export function applyTheme(doc: HTMLElement = document.documentElement, defaultScheme = 'rose') {
	try {
		const savedTheme = localStorage.getItem('theme');
		const systemTheme = window.matchMedia('(prefers-color-scheme: dark)').matches
			? 'dark'
			: 'light';
		const theme = savedTheme || systemTheme;

		const scheme = localStorage.getItem('scheme') || defaultScheme;

		const isDark = theme === 'dark';
		doc.classList.toggle('dark', isDark);
		doc.setAttribute('data-theme', theme);
		doc.setAttribute('data-scheme', scheme);
	} catch (e) {
		console.error('Theme initialization failed', e);
	}
}

export function setTheme(isDark: boolean) {
	const doc = document.documentElement;
	if (isDark) {
		doc.classList.add('dark');
		doc.setAttribute('data-theme', 'dark');
		localStorage.setItem('theme', 'dark');
	} else {
		doc.classList.remove('dark');
		doc.setAttribute('data-theme', 'light');
		localStorage.setItem('theme', 'light');
	}
}

export function setScheme(scheme: string, validNames: string[]) {
	if (!validNames.includes(scheme)) return;
	document.documentElement.setAttribute('data-scheme', scheme);
	localStorage.setItem('scheme', scheme);
}
