export {};

declare global {
	interface Turnstile {
		render(
			container: string | HTMLElement,
			options: {
				sitekey: string;
				callback?: (token: string) => void;
				appearance?: 'interaction-only' | 'always' | 'execute';
			}
		): string;

		reset(widgetId?: string): void;
	}

	interface Window {
		turnstile?: Turnstile;
	}
}
