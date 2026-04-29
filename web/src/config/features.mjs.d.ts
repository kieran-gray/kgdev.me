export interface FeatureFlag {
	enabled: boolean;
}

export interface Features {
	search: FeatureFlag;
	viewCounter: FeatureFlag;
	contact: FeatureFlag;
	og: FeatureFlag;
	mermaid: FeatureFlag;
	rss: FeatureFlag;
}

export const features: Features;
