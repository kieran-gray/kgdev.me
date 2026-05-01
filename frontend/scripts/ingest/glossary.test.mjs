import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { loadGlossary, resolveGlossaryTerms } from './glossary.mjs';

function withTmpDir(fn) {
	const dir = mkdtempSync(join(tmpdir(), 'ingest-glossary-'));
	try {
		fn(dir);
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
}

const VALID_ENTRY = `---
term: Durable Object
sources:
  - title: Cloudflare Durable Objects docs
    url: https://developers.cloudflare.com/durable-objects/
---

A single named instance of stateful compute.
`;

test('loadGlossary returns empty map when dir absent', () => {
	const map = loadGlossary('/nonexistent/glossary/dir');
	assert.equal(map.size, 0);
});

test('loadGlossary parses term, sources, and body', () => {
	withTmpDir((dir) => {
		writeFileSync(join(dir, 'durable-object.md'), VALID_ENTRY);
		const map = loadGlossary(dir);
		assert.equal(map.size, 1);
		const entry = map.get('durable-object');
		assert.equal(entry.term, 'Durable Object');
		assert.equal(entry.sources.length, 1);
		assert.equal(entry.sources[0].title, 'Cloudflare Durable Objects docs');
		assert.equal(entry.sources[0].url, 'https://developers.cloudflare.com/durable-objects/');
		assert.match(entry.definition, /single named instance/);
	});
});

test('loadGlossary handles entry with no sources', () => {
	withTmpDir((dir) => {
		writeFileSync(
			join(dir, 'foo.md'),
			`---
term: Foo
---

Body text.
`,
		);
		const map = loadGlossary(dir);
		assert.deepEqual(map.get('foo').sources, []);
	});
});

test('loadGlossary throws on missing term', () => {
	withTmpDir((dir) => {
		writeFileSync(join(dir, 'bad.md'), `---\nsources: []\n---\n\nbody\n`);
		assert.throws(() => loadGlossary(dir), /missing required "term"/);
	});
});

test('loadGlossary throws on empty body', () => {
	withTmpDir((dir) => {
		writeFileSync(join(dir, 'empty.md'), `---\nterm: Foo\n---\n\n`);
		assert.throws(() => loadGlossary(dir), /empty body/);
	});
});

test('resolveGlossaryTerms returns entries in order', () => {
	const map = new Map([
		['a', { term: 'A', definition: 'a', sources: [] }],
		['b', { term: 'B', definition: 'b', sources: [{ title: 't', url: 'https://x' }] }],
	]);
	const resolved = resolveGlossaryTerms('post', ['b', 'a'], map);
	assert.equal(resolved.length, 2);
	assert.equal(resolved[0].term, 'B');
	assert.equal(resolved[1].term, 'A');
});

test('resolveGlossaryTerms throws on unknown term', () => {
	assert.throws(
		() => resolveGlossaryTerms('post', ['nope'], new Map()),
		/unknown term "nope"/,
	);
});
