import { readFileSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { chunk } from './chunker.mjs';
import { readManifest, writeManifest, recordPost, previousEntry } from './manifest.mjs';
import { readEnv, embedBatch, upsertVectors, deleteVectorIds } from './cloudflare.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const POSTS_DIR = resolve(here, '../../src/content/posts');
const MANIFEST_PATH = resolve(here, 'manifest.json');

const INDEX_NAME = process.env.VECTORIZE_INDEX_NAME ?? 'blog-chunks';
const MODEL = process.env.EMBEDDING_MODEL ?? '@cf/baai/bge-base-en-v1.5';
const EMBED_BATCH = 50;
const UPSERT_BATCH = 100;

function parseFrontmatterTitle(source) {
	if (!source.startsWith('---\n')) return null;
	const end = source.indexOf('\n---\n', 4);
	if (end === -1) return null;
	const fm = source.slice(4, end);
	const match = fm.match(/^title:\s*['"](.+?)['"]\s*$/m);
	return match ? match[1] : null;
}

function sha256Hex(s) {
	return createHash('sha256').update(s).digest('hex');
}

function vectorId(slug, chunkId) {
	return `${slug}:${chunkId}`;
}

function discoverPosts(filter) {
	const files = readdirSync(POSTS_DIR).filter((f) => f.endsWith('.md'));
	const slugs = files.map((f) => basename(f, '.md'));
	if (!filter) return slugs;
	const found = slugs.filter((s) => filter.includes(s));
	const missing = filter.filter((s) => !slugs.includes(s));
	if (missing.length) throw new Error(`Unknown slug(s): ${missing.join(', ')}`);
	return found;
}

async function ingestPost(slug, opts) {
	const path = resolve(POSTS_DIR, `${slug}.md`);
	const source = readFileSync(path, 'utf8');
	const postVersion = sha256Hex(source);
	const title = parseFrontmatterTitle(source);

	const prev = previousEntry(opts.manifest, slug);
	if (prev?.post_version === postVersion && !opts.force) {
		console.log(`  ${slug}: unchanged (${postVersion.slice(0, 8)}), skipping`);
		return { slug, skipped: true };
	}

	const chunks = chunk(source);
	console.log(
		`  ${slug}: ${chunks.length} chunks` +
			(prev ? ` (was ${prev.chunk_count})` : '') +
			` @ ${postVersion.slice(0, 8)}`,
	);

	if (opts.dryRun) {
		return { slug, skipped: false, dryRun: true, chunkCount: chunks.length };
	}

	const embeddings = [];
	for (let i = 0; i < chunks.length; i += EMBED_BATCH) {
		const batch = chunks.slice(i, i + EMBED_BATCH);
		const vecs = await embedBatch(opts.cf, MODEL, batch.map((c) => c.text));
		embeddings.push(...vecs);
	}

	const records = chunks.map((c, i) => ({
		id: vectorId(slug, c.chunk_id),
		values: embeddings[i],
		metadata: {
			post_slug: slug,
			post_version: postVersion,
			post_title: title ?? slug,
			chunk_id: c.chunk_id,
			heading: c.heading,
			text: c.text,
			char_start: c.char_start,
			char_end: c.char_end,
		},
	}));

	for (let i = 0; i < records.length; i += UPSERT_BATCH) {
		await upsertVectors(opts.cf, INDEX_NAME, records.slice(i, i + UPSERT_BATCH));
	}

	if (prev && prev.chunk_count > chunks.length) {
		const stale = [];
		for (let i = chunks.length; i < prev.chunk_count; i++) {
			stale.push(vectorId(slug, i));
		}
		console.log(`  ${slug}: deleting ${stale.length} stale vector(s)`);
		await deleteVectorIds(opts.cf, INDEX_NAME, stale);
	}

	recordPost(opts.manifest, slug, {
		post_version: postVersion,
		chunk_count: chunks.length,
		ingested_at: new Date().toISOString(),
	});

	return { slug, skipped: false, chunkCount: chunks.length };
}

function printHelp() {
	console.log(
		[
			'Usage: node scripts/ingest/index.mjs [options] [slug...]',
			'',
			'Embeds blog post chunks and upserts them to Cloudflare Vectorize.',
			'',
			'Options:',
			'  --dry-run      Chunk and report, but do not call Cloudflare APIs.',
			'  --force        Re-ingest even if post_version is unchanged.',
			'  --help         Show this help.',
			'',
			'Env:',
			'  CLOUDFLARE_ACCOUNT_ID            Required (unless --dry-run).',
			'  CLOUDFLARE_RAG_INGEST_API_TOKEN Required (unless --dry-run). Needs Workers AI + Vectorize edit.',
			'  VECTORIZE_INDEX_NAME    Default: blog-chunks',
			'  EMBEDDING_MODEL         Default: @cf/baai/bge-base-en-v1.5',
		].join('\n'),
	);
}

async function main() {
	const { values, positionals } = parseArgs({
		options: {
			'dry-run': { type: 'boolean', default: false },
			force: { type: 'boolean', default: false },
			help: { type: 'boolean', default: false },
		},
		allowPositionals: true,
	});

	if (values.help) {
		printHelp();
		return;
	}

	const opts = {
		dryRun: values['dry-run'],
		force: values.force,
		manifest: readManifest(MANIFEST_PATH),
		cf: values['dry-run'] ? null : readEnv(),
	};

	const slugs = discoverPosts(positionals.length ? positionals : null);
	console.log(`Ingesting ${slugs.length} post(s) into ${INDEX_NAME}${opts.dryRun ? ' (dry run)' : ''}`);

	let ingested = 0;
	let skipped = 0;
	for (const slug of slugs) {
		const result = await ingestPost(slug, opts);
		if (result.skipped) skipped++;
		else ingested++;
	}

	if (!opts.dryRun) writeManifest(MANIFEST_PATH, opts.manifest);
	console.log(`Done. ingested=${ingested} skipped=${skipped}`);
}

main().catch((err) => {
	console.error(err.message ?? err);
	process.exit(1);
});
