export function readFlag(env, name, defaultValue) {
	const value = env?.[name];
	if (value == null) return defaultValue;
	return value !== 'false';
}

export function getFeatureFlags(env) {
	return {
		search: { enabled: readFlag(env, 'PUBLIC_FEATURE_SEARCH', true) },
		viewCounter: { enabled: readFlag(env, 'PUBLIC_FEATURE_VIEW_COUNTER', true) },
		contact: { enabled: readFlag(env, 'PUBLIC_FEATURE_CONTACT', true) },
		og: { enabled: readFlag(env, 'PUBLIC_FEATURE_OG', true) },
		mermaid: { enabled: readFlag(env, 'PUBLIC_FEATURE_MERMAID', true) },
		rss: { enabled: readFlag(env, 'PUBLIC_FEATURE_RSS', true) },
		projects: { enabled: readFlag(env, 'PUBLIC_FEATURE_PROJECTS', true) },
		books: { enabled: readFlag(env, 'PUBLIC_FEATURE_BOOKS', true) }
	};
}

const mergedEnv = {
	...(typeof process !== 'undefined' ? process.env : {}),
	...(typeof import.meta !== 'undefined' ? (import.meta.env ?? {}) : {})
};

export const features = getFeatureFlags(mergedEnv);
