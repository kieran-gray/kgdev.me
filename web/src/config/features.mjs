function readFlag(name, defaultValue) {
	const fromImportMeta =
		typeof import.meta !== 'undefined'
			? import.meta.env?.[name]
			: undefined;
	const fromProcess = process.env[name];
	const value = fromImportMeta ?? fromProcess;
	if (value == null) return defaultValue;
	return value !== 'false';
}

export const features = {
	search: { enabled: readFlag('PUBLIC_FEATURE_SEARCH', true) },
	viewCounter: { enabled: readFlag('PUBLIC_FEATURE_VIEW_COUNTER', true) },
	contact: { enabled: readFlag('PUBLIC_FEATURE_CONTACT', true) },
	og: { enabled: readFlag('PUBLIC_FEATURE_OG', true) },
	mermaid: { enabled: readFlag('PUBLIC_FEATURE_MERMAID', true) },
	rss: { enabled: readFlag('PUBLIC_FEATURE_RSS', true) }
};
