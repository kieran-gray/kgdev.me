import { getCollection, type CollectionEntry } from 'astro:content';
import { features } from '@/config/features';

export type BookEntry = CollectionEntry<'books'>;
export type Book = BookEntry['data'];

export async function getAllBooks(): Promise<Book[]> {
	if (!features.books.enabled) return [];
	const entries = await getCollection('books');
	return entries.map((entry) => entry.data);
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

export function formatBookDate(dateStr: string): string {
	const date = new Date(dateStr);
	return date.toLocaleDateString('en-US', {
		year: 'numeric',
		month: 'short',
		day: 'numeric'
	});
}

export function renderBookStars(rating: number): string {
	return '★'.repeat(rating) + '☆'.repeat(5 - rating);
}

export function getGoodreadsUrl(id: string): string {
	return `https://www.goodreads.com/book/show/${id}`;
}
