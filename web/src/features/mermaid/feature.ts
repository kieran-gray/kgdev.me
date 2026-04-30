import mermaid from 'astro-mermaid';
import type { BlogFeature } from '../_types';

export function createMermaidFeature(enabled: boolean): BlogFeature {
	return {
		name: 'mermaid',
		enabled,
		integration: enabled
			? mermaid({
					theme: 'neutral',
					autoTheme: true,
					enableLog: false,
					mermaidConfig: {
						flowchart: { curve: 'linear' }
					}
				})
			: false
	};
}
