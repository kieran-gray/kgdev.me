use leptos::prelude::*;
use leptos_router::components::A;

use crate::server_functions::posts::list_posts;
use crate::shared::PostSummary;

#[component]
pub fn PostsListPage() -> impl IntoView {
    let posts = Resource::new(|| (), |_| async move { list_posts().await });

    view! {
        <div class="space-y-8">
            <div class="px-6 flex flex-col gap-1">
                <span class="tech-label opacity-40">"SYSTEM_VIEW / BLOG_POSTS"</span>
                <h1 class="text-3xl font-bold tracking-tight">"POST_INDEX"</h1>
            </div>
            
            <Suspense fallback=|| view! { <div class="px-6"><p class="tech-label animate-pulse">"LOADING_DATA..."</p></div> }>
                {move || {
                    posts
                        .get()
                        .map(|res| match res {
                            Ok(list) => view! { <PostsTable posts=list /> }.into_any(),
                            Err(e) => {
                                view! {
                                    <div class="px-6">
                                        <div class="card-outer p-4 log-line-error font-mono text-sm">
                                            {format!("ERROR_LOG: {e}")}
                                        </div>
                                    </div>
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn PostsTable(posts: Vec<PostSummary>) -> impl IntoView {
    if posts.is_empty() {
        return view! { <div class="px-6"><div class="card-outer p-8 text-center tech-label opacity-40">"NO_RECORDS_FOUND"</div></div> }
            .into_any();
    }
    let total_records = posts.len();
    view! {
        <div class="border-y border-[var(--color-border)] bg-black/10 overflow-hidden">
            <table class="w-full text-sm border-collapse">
                <thead>
                    <tr class="bg-[var(--color-card-inner)]/50">
                        <th class="text-left px-6 py-3 tech-label opacity-50 border-b border-[var(--color-border)]">"SLUG"</th>
                        <th class="text-left px-4 py-3 tech-label opacity-50 border-b border-[var(--color-border)]">"TITLE"</th>
                        <th class="text-left px-4 py-3 tech-label opacity-50 border-b border-[var(--color-border)]">"STATUS"</th>
                        <th class="text-left px-4 py-3 tech-label opacity-50 border-b border-[var(--color-border)] text-center">"CHUNKS"</th>
                        <th class="text-right px-6 py-3 tech-label opacity-50 border-b border-[var(--color-border)]">"LAST_INGESTED"</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-[var(--color-border)]">
                    {posts
                        .into_iter()
                        .map(|p| {
                            let status_label = if p.manifest_post_version.is_none() {
                                ("NEVER_INGESTED", "text-amber-500/80")
                            } else if p.is_dirty {
                                ("DIRTY", "text-amber-500/80")
                            } else {
                                ("UP_TO_DATE", "text-emerald-500/80")
                            };
                            let chunks = p
                                .manifest_chunk_count
                                .map(|c| c.to_string())
                                .unwrap_or_else(|| "00".into());
                            let mut last = p
                                .manifest_ingested_at
                                .clone()
                                .unwrap_or_else(|| "N/A".into());
                            last.truncate(19);

                            let href = format!("/posts/{}", p.slug);
                            view! {
                                <tr class="hover:bg-[var(--color-accent)]/5 transition-colors group">
                                    <td class="px-6 py-3 font-mono text-xs">
                                        <A href=href.clone() attr:class="text-[var(--color-accent)] hover:underline">
                                            {format!("./{}", p.slug)}
                                        </A>
                                    </td>
                                    <td class="px-4 py-3 font-medium text-sm">{p.title}</td>
                                    <td class="px-4 py-3">
                                        <span class=format!("text-[10px] font-bold tracking-widest {}", status_label.1)>
                                            {status_label.0}
                                        </span>
                                    </td>
                                    <td class="px-4 py-3 font-mono text-xs text-center opacity-60">
                                        {format!("{:02}", chunks.parse::<i32>().unwrap_or(0))}
                                    </td>
                                    <td class="px-6 py-3 text-[10px] font-mono text-right opacity-40">
                                        {last}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
            <div class="px-6 py-2 bg-black/40 flex justify-between items-center border-t border-[var(--color-border)]">
                <span class="tech-label opacity-40 text-[9px]">{format!("TOTAL_RECORDS: {:03}", total_records)}</span>
                <span class="tech-label opacity-40 text-[9px]">"SYSTEM_STABLE // PAGE_01"</span>
            </div>
        </div>
    }
        .into_any()
}
