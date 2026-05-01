const API_BASE = 'https://api.cloudflare.com/client/v4';

export function readEnv() {
	const accountId = process.env.CLOUDFLARE_ACCOUNT_ID;
	const apiToken = process.env.CLOUDFLARE_RAG_INGEST_API_TOKEN;
	const kvNamespaceId = process.env.BLOG_POST_QA_CACHE_KV_NAMESPACE_ID;
	const missing = [];
	if (!accountId) missing.push('CLOUDFLARE_ACCOUNT_ID');
	if (!apiToken) missing.push('CLOUDFLARE_RAG_INGEST_API_TOKEN');
	if (!kvNamespaceId) missing.push('BLOG_POST_QA_CACHE_KV_NAMESPACE_ID');
	if (missing.length) {
		throw new Error(
			`Missing env vars: ${missing.join(', ')}. ` +
			'Token needs Workers AI + Vectorize edit + KV write permissions.',
		);
	}
	return { accountId, apiToken, kvNamespaceId };
}

async function cfFetch(url, init, ctx) {
	const res = await fetch(url, {
		...init,
		headers: {
			Authorization: `Bearer ${ctx.apiToken}`,
			...(init.headers ?? {}),
		},
	});
	const body = await res.text();
	if (!res.ok) {
		throw new Error(`${ctx.label}: ${res.status} ${res.statusText} — ${body.slice(0, 500)}`);
	}
	if (!body) return null;
	try {
		return JSON.parse(body);
	} catch {
		return body;
	}
}

export async function embedBatch(ctx, model, texts) {
	const url = `${API_BASE}/accounts/${ctx.accountId}/ai/run/${model}`;
	const json = await cfFetch(
		url,
		{
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ text: texts }),
		},
		{ ...ctx, label: 'workers-ai' },
	);
	if (!json?.success) {
		throw new Error(`workers-ai returned non-success: ${JSON.stringify(json?.errors ?? json)}`);
	}
	const data = json?.result?.data;
	if (!Array.isArray(data) || data.length !== texts.length) {
		throw new Error(`workers-ai returned ${data?.length ?? 0} embeddings for ${texts.length} inputs`);
	}
	return data;
}

export async function upsertVectors(ctx, indexName, records) {
	if (records.length === 0) return;
	const ndjson = records.map((r) => JSON.stringify(r)).join('\n') + '\n';
	const url = `${API_BASE}/accounts/${ctx.accountId}/vectorize/v2/indexes/${indexName}/upsert`;
	await cfFetch(
		url,
		{
			method: 'POST',
			headers: { 'content-type': 'application/x-ndjson' },
			body: ndjson,
		},
		{ ...ctx, label: 'vectorize-upsert' },
	);
}

export async function deleteVectorIds(ctx, indexName, ids) {
	if (ids.length === 0) return;
	const url = `${API_BASE}/accounts/${ctx.accountId}/vectorize/v2/indexes/${indexName}/delete-by-ids`;
	await cfFetch(
		url,
		{
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ ids }),
		},
		{ ...ctx, label: 'vectorize-delete' },
	);
}

export async function putKvJson(ctx, key, value) {
	const url = `${API_BASE}/accounts/${ctx.accountId}/storage/kv/namespaces/${ctx.kvNamespaceId}/values/${encodeURIComponent(key)}`;
	await cfFetch(
		url,
		{
			method: 'PUT',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(value),
		},
		{ ...ctx, label: 'kv-put' },
	);
}

export async function createMetadataIndex(ctx, indexName, propertyName, type = 'string') {
	const url = `${API_BASE}/accounts/${ctx.accountId}/vectorize/v2/indexes/${indexName}/metadata_index/create`;
	const json = await cfFetch(
		url,
		{
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ propertyName, indexType: type }),
		},
		{ ...ctx, label: 'vectorize-metadata-index' },
	);
	return json;
}
