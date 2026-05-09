mod commands;
mod components;
mod dialogs;
mod panels;
mod view;

use leptos::prelude::*;

use crate::server_functions::configuration::{get_configuration, get_pipeline_configurations};

use commands::ConfigTab;
use view::NewSettingsView;

#[component]
pub fn NewSettingsPage() -> impl IntoView {
    let (active_tab, set_active_tab) = signal(ConfigTab::PipelineConfigurations);
    let (refresh, set_refresh) = signal(0u32);
    let (busy, set_busy) = signal(false);
    let (status, set_status) = signal::<Option<(bool, String)>>(None);

    let configuration = Resource::new(
        move || refresh.get(),
        |_| async move { get_configuration().await.map_err(|e| e.to_string()) },
    );

    let pipeline_configurations = Resource::new(
        move || refresh.get(),
        |_| async move {
            get_pipeline_configurations()
                .await
                .map_err(|e| e.to_string())
        },
    );

    view! {
        <div class="space-y-8">
            <div class="px-6 flex flex-col gap-1">
                <span class="tech-label opacity-40">"SYSTEM_VIEW / CONFIGURATION"</span>
                <h1 class="text-3xl font-bold tracking-tight uppercase">"PIPELINE_LAB"</h1>
            </div>

            {move || {
                status.get().map(|(ok, msg)| {
                    let cls = if ok { "border-y border-x border-[var(--color-border)] p-4 text-emerald-400 bg-emerald-950/20" } else { "border-y border-x border-[var(--color-border)] p-4 log-line-error bg-red-950/20" };
                    view! { <div class=cls><div class="px-2"><span class="tech-label">{msg}</span></div></div> }
                })
            }}

            <Suspense fallback=|| view! { <div class="px-6"><p class="tech-label animate-pulse">"LOADING_CONFIGURATION_VIEW..."</p></div> }>
                {move || {
                    let config = configuration.get();
                    let pcs = pipeline_configurations.get();
                    match (config, pcs) {
                        (Some(Ok(config)), Some(Ok(pipeline_configs))) => view! {
                            <NewSettingsView
                                config=config
                                pipeline_configurations=pipeline_configs
                                active_tab=active_tab
                                set_active_tab=set_active_tab
                                busy=busy
                                set_busy=set_busy
                                set_status=set_status
                                set_refresh=set_refresh
                            />
                        }.into_any(),
                        (Some(Err(e)), _) | (_, Some(Err(e))) => view! {
                            <div class="px-6">
                                <div class="card-outer p-4 log-line-error font-mono text-sm">
                                    {format!("CONFIGURATION_VIEW_FAULT: {e}")}
                                </div>
                            </div>
                        }.into_any(),
                        _ => ().into_any(),
                    }
                }}
            </Suspense>
        </div>
    }
}
