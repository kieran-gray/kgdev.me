use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::server_fns::{load_settings, save_settings};
use crate::shared::SettingsDto;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let initial = Resource::new(|| (), |_| async move { load_settings().await });

    view! {
        <div class="space-y-6 max-w-3xl">
            <div class="flex flex-col border-b border-[var(--color-border)] pb-4">
                <span class="tech-label">"system.config"</span>
                <h1 class="text-2xl font-bold tracking-tight uppercase">"CONFIGURATION_PANEL"</h1>
                <p class="text-[10px] mt-2 font-mono opacity-50">
                    "LOCAL_STORAGE_REF: ./rag-admin/data/settings.toml // SYNC_REQUIRED_FOR_CHANGES"
                </p>
            </div>
            <Suspense fallback=|| view! { <p class="tech-label animate-pulse">"FETCHING_CONFIG..."</p> }>
                {move || {
                    initial
                        .get()
                        .map(|res| match res {
                            Ok(s) => view! { <SettingsForm initial=s /> }.into_any(),
                            Err(e) => {
                                view! {
                                    <div class="card-outer p-4 log-line-error font-mono text-sm">
                                        {format!("CONFIG_LOAD_FAULT: {e}")}
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
fn SettingsForm(initial: SettingsDto) -> impl IntoView {
    let (blog_url, set_blog_url) = signal(initial.blog_url);
    let (index, set_index) = signal(initial.vectorize_index_name);
    let (model, set_model) = signal(initial.embedding_model);
    let (account, set_account) = signal(initial.cloudflare_account_id);
    let (token, set_token) = signal(initial.cloudflare_api_token);
    let (kv_ns, set_kv_ns) = signal(initial.kv_namespace_id);

    let (status, set_status) = signal::<Option<(bool, String)>>(None);

    let on_save = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        set_status.set(None);
        let payload = SettingsDto {
            blog_url: blog_url.get(),
            vectorize_index_name: index.get(),
            embedding_model: model.get(),
            cloudflare_account_id: account.get(),
            cloudflare_api_token: token.get(),
            kv_namespace_id: kv_ns.get(),
        };
        spawn_local(async move {
            match save_settings(payload).await {
                Ok(()) => set_status.set(Some((true, "STATE_SYNC_COMPLETE".to_string()))),
                Err(e) => set_status.set(Some((false, format!("SYNC_FAULT: {e}")))),
            }
        });
    };

    view! {
        <form on:submit=on_save class="card-outer p-6 space-y-6">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <Field label="BLOG_BASE_URL" hint="TARGET_URI (e.g. https://kgdev.me)">
                    <input
                        class="input font-mono text-sm"
                        prop:value=blog_url
                        on:input=move |e| set_blog_url.set(event_target_value(&e))
                    />
                </Field>
                <Field label="VECTORIZE_INDEX" hint="CLOUDFLARE_VECTORIZE_ID">
                    <input
                        class="input font-mono text-sm"
                        prop:value=index
                        on:input=move |e| set_index.set(event_target_value(&e))
                    />
                </Field>
                <Field label="EMBEDDING_MODEL" hint="WORKERS_AI_MODEL_ID">
                    <input
                        class="input font-mono text-sm"
                        prop:value=model
                        on:input=move |e| set_model.set(event_target_value(&e))
                    />
                </Field>
                <Field label="CF_ACCOUNT_ID" hint="CLOUDFLARE_AUTH_CONTEXT">
                    <input
                        class="input font-mono text-sm"
                        prop:value=account
                        on:input=move |e| set_account.set(event_target_value(&e))
                    />
                </Field>
                <Field label="CF_API_TOKEN" hint="ENCRYPTED_AUTH_TOKEN">
                    <input
                        class="input font-mono text-sm"
                        type="password"
                        prop:value=token
                        on:input=move |e| set_token.set(event_target_value(&e))
                    />
                </Field>
                <Field label="KV_NAMESPACE_ID" hint="STORAGE_CACHE_ID">
                    <input
                        class="input font-mono text-sm"
                        prop:value=kv_ns
                        on:input=move |e| set_kv_ns.set(event_target_value(&e))
                    />
                </Field>
            </div>
            <div class="flex items-center justify-between pt-4 border-t border-[var(--color-border)]">
                <div class="flex items-center gap-4">
                    <button class="btn btn-primary px-8" type="submit">"SAVE_CHANGES"</button>
                    {move || {
                        status
                            .get()
                            .map(|(ok, msg)| {
                                let cls = if ok { "text-emerald-500" } else { "text-red-500" };
                                view! { <span class=format!("tech-label {}", cls)>{msg}</span> }
                            })
                    }}
                </div>
                <span class="tech-label opacity-30">"CHECKSUM_VERIFIED"</span>
            </div>
        </form>
    }
}

#[component]
fn Field(label: &'static str, hint: &'static str, children: Children) -> impl IntoView {
    view! {
        <label class="block space-y-1.5">
            <div class="tech-label opacity-70">
                {label}
            </div>
            {children()}
            <div class="tech-label text-[9px] opacity-40">
                {format!("> {}", hint)}
            </div>
        </label>
    }
}
