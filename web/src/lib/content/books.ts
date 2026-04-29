import fs from 'node:fs';
import path from 'node:path';

export interface Book {
	id: string;
	title: string;
	author: string;
	rating: number;
	dateRead: string | null;
	shelf: string;
	readCount: number;
}

export function parseBooks(): Book[] {
	const csvPath = path.resolve('./src/content/goodreads_library_export.csv');
	const csvData = fs.readFileSync(csvPath, 'utf-8');
	const lines = csvData.split(/\r?\n/);
	const books: Book[] = [];

	// Skip header
	for (let i = 1; i < lines.length; i++) {
		const lineText = lines[i];
		if (lineText === undefined) continue;
		const line = lineText.trim();
		if (!line) continue;

		const parts = parseCsvLine(line);
		if (parts.length < 22) continue;

		books.push({
			id: parts[0] ?? '',
			title: cleanTitle(parts[1] ?? ''),
			author: parts[2] ?? '',
			rating: parseInt(parts[7] ?? '0') || 0,
			dateRead: parts[13] ? parts[13].replace(/\//g, '-') : null,
			shelf: parts[17] ?? '',
			readCount: parseInt(parts[21] ?? '0') || 0
		});
	}

	return books;
}

function cleanTitle(title: string): string {
	// Remove (Series, #1) or similar suffixes common in Goodreads
	return title.replace(/\s*\([^)]+\)$/, '').trim();
}

function parseCsvLine(line: string): string[] {
	const result = [];
	let current = '';
	let inQuotes = false;

	for (let i = 0; i < line.length; i++) {
		const char = line[i];

		if (char === '"') {
			if (inQuotes && line[i + 1] === '"') {
				// Escaped quote
				current += '"';
				i++;
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

export function getCurrentlyReading(books: Book[]): Book[] {
	return books.filter((book) => book.shelf === 'currently-reading');
}

export function getReadHistory(books: Book[]): Record<string, Book[]> {
	const readBooks = books.filter((book) => book.shelf === 'read' && book.dateRead);

	// Sort by date read descending
	readBooks.sort((a, b) => {
		return new Date(b.dateRead!).getTime() - new Date(a.dateRead!).getTime();
	});

	const history: Record<string, Book[]> = {};
	readBooks.forEach((book) => {
		const year = new Date(book.dateRead!).getFullYear().toString();
		if (!history[year]) {
			history[year] = [];
		}
		history[year].push(book);
	});

	return history;
}
