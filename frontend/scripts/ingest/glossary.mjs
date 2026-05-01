import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { resolve, basename } from 'node:path';

function unquote(value) {
	const trimmed = value.trim();
	if (
		(trimmed.startsWith("'") && trimmed.endsWith("'")) ||
		(trimmed.startsWith('"') && trimmed.endsWith('"'))
	) {
		return trimmed.slice(1, -1);
	}
	return trimmed;
}

function parseGlossaryFrontmatter(source, slug) {
	if (!source.startsWith('---\n')) {
		throw new Error(`glossary/${slug}: missing frontmatter`);
	}
	const end = source.indexOf('\n---\n', 4);
	if (end === -1) {
		throw new Error(`glossary/${slug}: unterminated frontmatter`);
	}
	const fm = source.slice(4, end);
	const body = source.slice(end + 5).trim();

	const lines = fm.split('\n');
	let term = null;
	const sources = [];
	let i = 0;
	while (i < lines.length) {
		const line = lines[i];
		const termMatch = line.match(/^term:\s*(.+)$/);
		if (termMatch) {
			term = unquote(termMatch[1]);
			i++;
			continue;
		}
		if (line.match(/^sources:\s*$/)) {
			i++;
			while (i < lines.length && lines[i].startsWith('  ')) {
				const itemMatch = lines[i].match(/^\s*-\s*title:\s*(.+)$/);
				if (!itemMatch) {
					throw new Error(`glossary/${slug}: expected "- title: ..." at line ${i + 1}`);
				}
				const title = unquote(itemMatch[1]);
				i++;
				if (i >= lines.length) {
					throw new Error(`glossary/${slug}: source missing url for "${title}"`);
				}
				const urlMatch = lines[i].match(/^\s+url:\s*(.+)$/);
				if (!urlMatch) {
					throw new Error(`glossary/${slug}: source missing url for "${title}"`);
				}
				sources.push({ title, url: unquote(urlMatch[1]) });
				i++;
			}
			continue;
		}
		i++;
	}

	if (!term) {
		throw new Error(`glossary/${slug}: missing required "term" in frontmatter`);
	}
	if (!body) {
		throw new Error(`glossary/${slug}: empty body`);
	}

	return { term, sources, definition: body };
}

export function loadGlossary(dir) {
	if (!existsSync(dir)) return new Map();
	const map = new Map();
	for (const file of readdirSync(dir)) {
		if (!file.endsWith('.md')) continue;
		const slug = basename(file, '.md');
		const source = readFileSync(resolve(dir, file), 'utf8');
		map.set(slug, parseGlossaryFrontmatter(source, slug));
	}
	return map;
}

export function resolveGlossaryTerms(slug, termSlugs, glossaryMap) {
	const resolved = [];
	for (const termSlug of termSlugs) {
		const entry = glossaryMap.get(termSlug);
		if (!entry) {
			throw new Error(
				`posts/${slug}: glossaryTerms references unknown term "${termSlug}". ` +
					`Add frontend/src/content/glossary/${termSlug}.md or remove the reference.`,
			);
		}
		resolved.push({
			term: entry.term,
			definition: entry.definition,
			sources: entry.sources,
		});
	}
	return resolved;
}
