import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { readManifest, writeManifest, recordPost, previousEntry } from './manifest.mjs';

function withTmp(fn) {
	const dir = mkdtempSync(join(tmpdir(), 'ingest-manifest-'));
	try {
		fn(dir);
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
}

test('readManifest returns empty when file is absent', () => {
	withTmp((dir) => {
		const m = readManifest(join(dir, 'missing.json'));
		assert.deepEqual(m, { version: 1, posts: {} });
	});
});

test('readManifest handles malformed file as empty', () => {
	withTmp((dir) => {
		const path = join(dir, 'bad.json');
		writeFileSync(path, 'null');
		const m = readManifest(path);
		assert.deepEqual(m, { version: 1, posts: {} });
	});
});

test('write then read round-trips', () => {
	withTmp((dir) => {
		const path = join(dir, 'm.json');
		const m = { version: 1, posts: {} };
		recordPost(m, 'b-slug', { post_version: 'v2', chunk_count: 5, ingested_at: '2026-04-30T00:00:00Z' });
		recordPost(m, 'a-slug', { post_version: 'v1', chunk_count: 3, ingested_at: '2026-04-29T00:00:00Z' });
		writeManifest(path, m);

		const read = readManifest(path);
		assert.equal(read.posts['a-slug'].post_version, 'v1');
		assert.equal(read.posts['b-slug'].chunk_count, 5);

		const onDisk = readFileSync(path, 'utf8');
		const aIdx = onDisk.indexOf('"a-slug"');
		const bIdx = onDisk.indexOf('"b-slug"');
		assert.ok(aIdx > 0 && bIdx > 0 && aIdx < bIdx, 'posts must be alphabetically ordered on disk');
		assert.ok(onDisk.endsWith('\n'));
	});
});

test('previousEntry returns null for unknown slug', () => {
	const m = { version: 1, posts: { 'known': { post_version: 'v1', chunk_count: 1 } } };
	assert.equal(previousEntry(m, 'unknown'), null);
	assert.equal(previousEntry(m, 'known').post_version, 'v1');
});
