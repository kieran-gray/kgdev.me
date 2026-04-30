import type { APIRoute } from 'astro';
import { renderDefaultCard } from './card';

export const GET: APIRoute = async () => {
	const png = await renderDefaultCard();
	return new Response(png, {
		headers: {
			'Content-Type': 'image/png',
			'Cache-Control': 'public, max-age=31536000, immutable'
		}
	});
};
