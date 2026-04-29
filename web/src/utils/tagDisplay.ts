const TAG_DISPLAY: Record<string, string> = {
	rust: 'Rust',
	typescript: 'TypeScript',
	python: 'Python',
	sql: 'SQL'
};

export function getTagDisplay(tag: string): string {
	return TAG_DISPLAY[tag] ?? tag;
}
