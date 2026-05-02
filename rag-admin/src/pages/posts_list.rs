use leptos::prelude::*;
use leptos_router::components::A;

use crate::server_fns::list_posts;
use crate::shared::PostSummary;

#[component]
pub fn PostsListPage() -> impl IntoView {
    let posts = Resource::new(|| (), |_| async move { list_posts().await });

    view! {
        <div class="space-y-6">
            <div class="flex items-end justify-between border-b border-[var(--color-border)] pb-2">
                <div class="flex flex-col">
                    <h1 class="text-2xl font-bold tracking-tight">"BLOG_POSTS"</h1>
                        <p class="text-[10px] mt-2 font-mono opacity-50">
                            "MANAGE_POST_EMBEDDINGS"
                        </p>
                </div>
            </div>
            <Suspense fallback=|| view! { <p class="tech-label animate-pulse">"LOADING_DATA..."</p> }>
                {move || {
                    posts
                        .get()
                        .map(|res| match res {
                            Ok(list) => view! { <PostsTable posts=list /> }.into_any(),
                            Err(e) => {
                                view! {
                                    <div class="card-outer p-4 log-line-error font-mono text-sm">
                                        {format!("ERROR_LOG: {e}")}
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
        return view! { <div class="card-outer p-4 tech-label">"NO_RECORDS_FOUND"</div> }
            .into_any();
    }
    let total_records = posts.len();
    view! {
        <div class="card-outer overflow-hidden">
            <table class="w-full text-sm border-collapse">
                <thead>
                    <tr style="background-color: var(--color-card-inner);">
                        <th class="text-left px-4 py-2 tech-label border-b border-[var(--color-border)]">"SLUG"</th>
                        <th class="text-left px-4 py-2 tech-label border-b border-[var(--color-border)]">"TITLE"</th>
                        <th class="text-left px-4 py-2 tech-label border-b border-[var(--color-border)]">"STATUS"</th>
                        <th class="text-left px-4 py-2 tech-label border-b border-[var(--color-border)] text-center">"CHUNKS"</th>
                        <th class="text-right px-4 py-2 tech-label border-b border-[var(--color-border)]">"LAST_INGESTED"</th>
                    </tr>
                </thead>
                <tbody>
                    {posts
                        .into_iter()
                        .map(|p| {
                            let status_label = if p.manifest_post_version.is_none() {
                                ("NEVER_INGESTED", "text-amber-500")
                            } else if p.is_dirty {
                                ("DIRTY", "text-amber-500")
                            } else {
                                ("UP_TO_DATE", "text-emerald-500")
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
                                <tr class="hover:bg-[var(--color-card-inner)] transition-colors group">
                                    <td class="px-4 py-2 font-mono text-xs border-b border-[var(--color-border)]">
                                        <A href=href.clone() attr:class="text-[var(--color-accent)]">
                                            {format!("./{}", p.slug)}
                                        </A>
                                    </td>
                                    <td class="px-4 py-2 font-medium border-b border-[var(--color-border)]">{p.title}</td>
                                    <td class="px-4 py-2 border-b border-[var(--color-border)]">
                                        <span class=format!("text-[10px] font-bold tracking-widest {}", status_label.1)>
                                            {status_label.0}
                                        </span>
                                    </td>
                                    <td class="px-4 py-2 font-mono text-xs text-center border-b border-[var(--color-border)]">
                                        {format!("{:02}", chunks.parse::<i32>().unwrap_or(0))}
                                    </td>
                                    <td class="px-4 py-2 text-[10px] font-mono text-right border-b border-[var(--color-border)]" style="color: var(--color-muted);">
                                        {last}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
            <div class="px-4 py-1 bg-[var(--color-page-bg)] flex justify-between items-center border-t border-[var(--color-border)]">
                <span class="tech-label opacity-50">{format!("TOTAL_RECORDS: {:03}", total_records)}</span>
                <span class="tech-label opacity-50">"PAGE_01_OF_01"</span>
            </div>
        </div>
    }
        .into_any()
}
