import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const DEFAULT_INPUT = 'src/content/goodreads_library_export.csv';
const DEFAULT_OUTPUT = 'src/features/books/content/books.json';

const [, , inputArg, outputArg] = process.argv;
const inputPath = resolve(inputArg ?? DEFAULT_INPUT);
const outputPath = resolve(outputArg ?? DEFAULT_OUTPUT);

function cleanTitle(title) {
	return title.replace(/\s*\([^)]+\)$/, '').trim();
}

function parseCsvLine(line) {
	const result = [];
	let current = '';
	let inQuotes = false;

	for (let i = 0; i < line.length; i += 1) {
		const char = line[i];

		if (char === '"') {
			if (inQuotes && line[i + 1] === '"') {
				current += '"';
				i += 1;
			} else {
				inQuotes = !inQuotes;
			}
		} else if (char === ',' && !inQuotes) {
			result.push(current);
			current = '';
		} else {
			current += char;
		}
	}

	result.push(current);
	return result;
}

function convertGoodreadsCsv(csv) {
	const lines = csv.split(/\r?\n/);
	const books = [];

	for (let i = 1; i < lines.length; i += 1) {
		const lineText = lines[i];
		if (!lineText) continue;
		const line = lineText.trim();
		if (!line) continue;

		const parts = parseCsvLine(line);
		if (parts.length < 22) continue;

		books.push({
			id: parts[0] ?? '',
			title: cleanTitle(parts[1] ?? ''),
			author: parts[2] ?? '',
			rating: Number.parseInt(parts[7] ?? '0', 10) || 0,
			dateRead: parts[13] ? parts[13].replace(/\//g, '-') : null,
			shelf: parts[17] ?? '',
			readCount: Number.parseInt(parts[21] ?? '0', 10) || 0
		});
	}

	return books;
}

const csv = readFileSync(inputPath, 'utf8');
const books = convertGoodreadsCsv(csv);

writeFileSync(outputPath, `${JSON.stringify(books, null, 2)}\n`);

console.log(`Converted ${books.length} books`);
console.log(`Input:  ${inputPath}`);
console.log(`Output: ${outputPath}`);
