import type { AstroIntegration } from 'astro';

export interface FeatureRoute {
	pattern: string;
	entrypoint: string;
	prerender?: boolean;
}

export interface BlogFeature {
	name: string;
	enabled: boolean;
	routes?: FeatureRoute[];
	integration?: AstroIntegration | false;
	postbuild?: () => void | Promise<void>;
	sitemapExclude?: (pathname: string) => boolean;
}
