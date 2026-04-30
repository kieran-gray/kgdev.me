import { setTheme, setScheme } from '@/scripts/theme-runtime';

const root = document.documentElement;
const schemeNames = (root.dataset.schemeList || '').split('|').filter(Boolean);

function updateSchemeUI(scheme: string) {
	document.querySelectorAll<HTMLElement>('.scheme-menu').forEach((menu) => {
		menu.querySelectorAll<HTMLElement>('.scheme-option').forEach((btn) => {
			const isActive = btn.dataset.scheme === scheme;
			btn.setAttribute('aria-checked', isActive ? 'true' : 'false');
			const check = btn.querySelector('svg');
			if (check) check.classList.toggle('hidden', !isActive);
		});
	});
}

function applySchemeAndUpdate(scheme: string) {
	setScheme(scheme, schemeNames);
	updateSchemeUI(scheme);
}

function closeAllSchemeMenus() {
	document
		.querySelectorAll('#scheme-button-desktop, #scheme-button-mobile')
		.forEach((b) => b.setAttribute('aria-expanded', 'false'));
	document.querySelectorAll('.scheme-menu').forEach((m) => m.classList.add('hidden'));
}

function closeMobileMenu() {
	const btn = document.getElementById('mobile-menu-button');
	const menu = document.getElementById('mobile-menu');
	const backdrop = document.getElementById('mobile-menu-backdrop');
	const openIcon = document.getElementById('mobile-menu-icon-open');
	const closeIcon = document.getElementById('mobile-menu-icon-close');
	if (!btn || !menu || !backdrop) return;
	btn.setAttribute('aria-expanded', 'false');
	menu.classList.add('scale-95', 'opacity-0', 'pointer-events-none');
	menu.classList.remove('scale-100', 'opacity-100');
	backdrop.classList.add('opacity-0', 'pointer-events-none');
	backdrop.classList.remove('opacity-100');
	openIcon?.classList.remove('hidden');
	closeIcon?.classList.add('hidden');
	window.setTimeout(() => {
		if (btn.getAttribute('aria-expanded') === 'false') {
			menu.classList.add('hidden');
			backdrop.classList.add('hidden');
		}
	}, 200);
}

function openMobileMenu() {
	const btn = document.getElementById('mobile-menu-button');
	const menu = document.getElementById('mobile-menu');
	const backdrop = document.getElementById('mobile-menu-backdrop');
	const openIcon = document.getElementById('mobile-menu-icon-open');
	const closeIcon = document.getElementById('mobile-menu-icon-close');
	if (!btn || !menu || !backdrop) return;
	btn.setAttribute('aria-expanded', 'true');
	menu.classList.remove('hidden');
	backdrop.classList.remove('hidden');
	requestAnimationFrame(() => {
		menu.classList.remove('scale-95', 'opacity-0', 'pointer-events-none');
		menu.classList.add('scale-100', 'opacity-100');
		backdrop.classList.remove('opacity-0', 'pointer-events-none');
		backdrop.classList.add('opacity-100');
	});
	openIcon?.classList.add('hidden');
	closeIcon?.classList.remove('hidden');
}

function toggleMobileMenu() {
	const btn = document.getElementById('mobile-menu-button');
	if (!btn) return;
	if (btn.getAttribute('aria-expanded') === 'true') closeMobileMenu();
	else openMobileMenu();
}

function setupNavbar() {
	const isDarkTheme = document.documentElement.classList.contains('dark');
	const initialScheme = document.documentElement.getAttribute('data-scheme') || '';

	document.querySelectorAll<HTMLInputElement>('.light-switch').forEach((sw) => {
		sw.checked = isDarkTheme;
	});
	updateSchemeUI(initialScheme);
	closeMobileMenu();
}

let delegatedWired = false;
function wireDelegation() {
	if (delegatedWired) return;
	delegatedWired = true;

	document.addEventListener('click', (e) => {
		const target = e.target as HTMLElement | null;
		if (!target) return;

		const schemeBtn = target.closest('#scheme-button-desktop, #scheme-button-mobile');
		if (schemeBtn) {
			e.stopPropagation();
			const menuId = schemeBtn.getAttribute('aria-controls');
			const menu = menuId ? document.getElementById(menuId) : null;
			if (!menu) return;
			const expanded = schemeBtn.getAttribute('aria-expanded') === 'true';
			closeAllSchemeMenus();
			if (!expanded) {
				schemeBtn.setAttribute('aria-expanded', 'true');
				menu.classList.remove('hidden');
			}
			return;
		}

		const schemeOption = target.closest<HTMLElement>('.scheme-option');
		if (schemeOption) {
			e.stopPropagation();
			const scheme = schemeOption.dataset.scheme;
			if (scheme) applySchemeAndUpdate(scheme);
			closeAllSchemeMenus();
			return;
		}

		if (target.closest('#mobile-menu-button')) {
			e.stopPropagation();
			toggleMobileMenu();
			return;
		}

		if (target.closest('#mobile-menu-backdrop')) {
			closeMobileMenu();
			return;
		}

		if (!target.closest('.scheme-menu')) closeAllSchemeMenus();
	});

	document.addEventListener('change', (e) => {
		const target = e.target as HTMLInputElement | null;
		if (target && target.classList && target.classList.contains('light-switch')) {
			setTheme(target.checked);
			document.querySelectorAll<HTMLInputElement>('.light-switch').forEach((sw) => {
				sw.checked = target.checked;
			});
		}
	});

	document.addEventListener('keydown', (e) => {
		if (e.key === 'Escape') {
			closeAllSchemeMenus();
			closeMobileMenu();
		}
	});
}

wireDelegation();
setupNavbar();
document.addEventListener('astro:page-load', setupNavbar);
