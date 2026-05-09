use leptos::prelude::*;

use crate::shared::{
    AiProviderDto, ConfigurationDto, EmbeddingModelDto, GenerationModelDto,
    PipelineConfigurationDto, VectorIndexDto, VectorStoreProviderDto,
};

use super::commands::{provider_name_for, short_uuid, vector_store_provider_name_for};
use super::components::{EmptyState, MetaPill, PanelHeader};
use super::dialogs::DeleteDialog;

#[component]
pub fn ProvidersPanel(
    config: StoredValue<ConfigurationDto>,
    busy: ReadSignal<bool>,
    on_add: Box<dyn Fn(leptos::ev::MouseEvent) + Send + Sync>,
    on_edit_ai: Box<dyn Fn(AiProviderDto) + Send + Sync>,
    on_edit_vs: Box<dyn Fn(VectorStoreProviderDto) + Send + Sync>,
    set_delete_dialog: WriteSignal<Option<DeleteDialog>>,
) -> impl IntoView {
    let on_edit_ai = StoredValue::new(on_edit_ai);
    let on_edit_vs = StoredValue::new(on_edit_vs);
    view! {
        <section class="space-y-6">
            <PanelHeader
                title="PROVIDER_REGISTRY"
                description="Providers anchor AI models and vector store backends. Add one provider per external system."
                action_label="ADD_PROVIDER"
                action_disabled=move || busy.get()
                on_action=Box::new(on_add)
            />
            <div class="space-y-3">
                <div class="tech-label opacity-50 text-xs">"AI_PROVIDERS"</div>
                {config.with_value(|cfg| {
                    if cfg.ai_providers.is_empty() {
                        view! {
                            <EmptyState message="No AI providers yet. Add one so embedding and generation models have somewhere to attach." />
                        }.into_any()
                    } else {
                        cfg.ai_providers.iter().map(|provider| {
                            let embedding_count = cfg.embedding_models.iter().filter(|m| m.provider_id == provider.provider_id).count();
                            let generation_count = cfg.generation_models.iter().filter(|m| m.provider_id == provider.provider_id).count();
                            let provider_for_edit = provider.clone();
                            let provider_for_delete = provider.clone();
                            view! {
                                <div class="card-outer p-4 flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                                    <div class="space-y-2">
                                        <h3 class="text-lg font-semibold">{provider.name.clone()}</h3>
                                        <div class="flex gap-2 flex-wrap">
                                            <MetaPill label=format!("{embedding_count} embedding") />
                                            <MetaPill label=format!("{generation_count} generation") />
                                            <MetaPill label=short_uuid(provider.provider_id) />
                                        </div>
                                    </div>
                                    <div class="flex gap-2 flex-wrap">
                                        <button
                                            class="btn"
                                            disabled=busy
                                            on:click=move |_| on_edit_ai.with_value(|f| f(provider_for_edit.clone()))
                                        >
                                            "EDIT"
                                        </button>
                                        <button
                                            class="btn"
                                            disabled=busy
                                            on:click=move |_| set_delete_dialog.set(Some(DeleteDialog::AiProvider(provider_for_delete.clone())))
                                        >
                                            "DELETE"
                                        </button>
                                    </div>
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                })}
            </div>
            <div class="space-y-3">
                <div class="tech-label opacity-50 text-xs">"VECTOR_STORE_PROVIDERS"</div>
                {config.with_value(|cfg| {
                    if cfg.vector_store_providers.is_empty() {
                        view! {
                            <EmptyState message="No vector store providers yet. Add one so indexes have a backend to attach to." />
                        }.into_any()
                    } else {
                        cfg.vector_store_providers.iter().map(|provider| {
                            let index_count = cfg.vector_indexes.iter().filter(|i| i.vector_store_provider_id == provider.provider_id).count();
                            let provider_for_edit = provider.clone();
                            let provider_for_delete = provider.clone();
                            view! {
                                <div class="card-outer p-4 flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                                    <div class="space-y-2">
                                        <h3 class="text-lg font-semibold">{provider.name.clone()}</h3>
                                        <div class="flex gap-2 flex-wrap">
                                            <MetaPill label=format!("{index_count} indexes") />
                                            <MetaPill label=short_uuid(provider.provider_id) />
                                        </div>
                                    </div>
                                    <div class="flex gap-2 flex-wrap">
                                        <button
                                            class="btn"
                                            disabled=busy
                                            on:click=move |_| on_edit_vs.with_value(|f| f(provider_for_edit.clone()))
                                        >
                                            "EDIT"
                                        </button>
                                        <button
                                            class="btn"
                                            disabled=busy
                                            on:click=move |_| set_delete_dialog.set(Some(DeleteDialog::VectorStoreProvider(provider_for_delete.clone())))
                                        >
                                            "DELETE"
                                        </button>
                                    </div>
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                })}
            </div>
        </section>
    }
}

#[component]
pub fn EmbeddingModelsPanel(
    config: StoredValue<ConfigurationDto>,
    busy: ReadSignal<bool>,
    on_add: Box<dyn Fn(leptos::ev::MouseEvent) + Send + Sync>,
    on_edit: Box<dyn Fn(EmbeddingModelDto) + Send + Sync>,
    set_delete_dialog: WriteSignal<Option<DeleteDialog>>,
) -> impl IntoView {
    let on_edit = StoredValue::new(on_edit);
    view! {
        <section class="space-y-4">
            <PanelHeader
                title="EMBEDDING_MODEL_REGISTRY"
                description="Keep embedding models lean and measurable: provider, model id, dimensions."
                action_label="ADD_EMBEDDING_MODEL"
                action_disabled=move || busy.get() || config.with_value(|cfg| cfg.ai_providers.is_empty())
                on_action=Box::new(on_add)
            />
            <div class="space-y-3">
                {config.with_value(|cfg| {
                    if cfg.embedding_models.is_empty() {
                        view! {
                            <EmptyState message="No embedding models yet. Add a provider first, then register models against it." />
                        }.into_any()
                    } else {
                        cfg.embedding_models.iter().map(|model| {
                            let provider_name = provider_name_for(&cfg.ai_providers, model.provider_id);
                            let model_for_edit = model.clone();
                            let model_for_delete = model.clone();
                            view! {
                                <div class="card-outer p-4 flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                                    <div class="space-y-2">
                                        <h3 class="text-lg font-semibold break-all">{model.model.clone()}</h3>
                                        <div class="flex gap-2 flex-wrap">
                                            <MetaPill label=provider_name />
                                            <MetaPill label=format!("{} dims", model.dimensions) />
                                            <MetaPill label=short_uuid(model.embedding_model_id) />
                                        </div>
                                    </div>
                                    <div class="flex gap-2 flex-wrap">
                                        <button
                                            class="btn"
                                            disabled=busy
                                            on:click=move |_| on_edit.with_value(|f| f(model_for_edit.clone()))
                                        >
                                            "EDIT"
                                        </button>
                                        <button
                                            class="btn"
                                            disabled=busy
                                            on:click=move |_| set_delete_dialog.set(Some(DeleteDialog::EmbeddingModel(model_for_delete.clone())))
                                        >
                                            "DELETE"
                                        </button>
                                    </div>
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                })}
            </div>
        </section>
    }
}

#[component]
pub fn GenerationModelsPanel(
    config: StoredValue<ConfigurationDto>,
    busy: ReadSignal<bool>,
    on_add: Box<dyn Fn(leptos::ev::MouseEvent) + Send + Sync>,
    on_edit: Box<dyn Fn(GenerationModelDto) + Send + Sync>,
    set_delete_dialog: WriteSignal<Option<DeleteDialog>>,
) -> impl IntoView {
    let on_edit = StoredValue::new(on_edit);
    view! {
        <section class="space-y-4">
            <PanelHeader
                title="GENERATION_MODEL_REGISTRY"
                description="These models drive synthesis and chat-style generation work."
                action_label="ADD_GENERATION_MODEL"
                action_disabled=move || busy.get() || config.with_value(|cfg| cfg.ai_providers.is_empty())
                on_action=Box::new(on_add)
            />
            <div class="space-y-3">
                {config.with_value(|cfg| {
                    if cfg.generation_models.is_empty() {
                        view! {
                            <EmptyState message="No generation models yet. Add a provider first, then register the models you want to test." />
                        }.into_any()
                    } else {
                        cfg.generation_models.iter().map(|model| {
                            let provider_name = provider_name_for(&cfg.ai_providers, model.provider_id);
                            let model_for_edit = model.clone();
                            let model_for_delete = model.clone();
                            view! {
                                <div class="card-outer p-4 flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                                    <div class="space-y-2">
                                        <h3 class="text-lg font-semibold break-all">{model.model.clone()}</h3>
                                        <div class="flex gap-2 flex-wrap">
                                            <MetaPill label=provider_name />
                                            <MetaPill label=short_uuid(model.generation_model_id) />
                                        </div>
                                    </div>
                                    <div class="flex gap-2 flex-wrap">
                                        <button
                                            class="btn"
                                            disabled=busy
                                            on:click=move |_| on_edit.with_value(|f| f(model_for_edit.clone()))
                                        >
                                            "EDIT"
                                        </button>
                                        <button
                                            class="btn"
                                            disabled=busy
                                            on:click=move |_| set_delete_dialog.set(Some(DeleteDialog::GenerationModel(model_for_delete.clone())))
                                        >
                                            "DELETE"
                                        </button>
                                    </div>
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                })}
            </div>
        </section>
    }
}

#[component]
pub fn VectorIndexesPanel(
    config: StoredValue<ConfigurationDto>,
    busy: ReadSignal<bool>,
    on_add: Box<dyn Fn(leptos::ev::MouseEvent) + Send + Sync>,
    on_edit: Box<dyn Fn(VectorIndexDto) + Send + Sync>,
    set_delete_dialog: WriteSignal<Option<DeleteDialog>>,
) -> impl IntoView {
    let on_edit = StoredValue::new(on_edit);
    view! {
        <section class="space-y-4">
            <PanelHeader
                title="VECTOR_INDEX_REGISTRY"
                description="Indexes live inside a vector store provider. Add a provider first, then register the indexes you want to target."
                action_label="ADD_VECTOR_INDEX"
                action_disabled=move || busy.get() || config.with_value(|cfg| cfg.vector_store_providers.is_empty())
                on_action=Box::new(on_add)
            />
            <div class="space-y-3">
                {config.with_value(|cfg| {
                    if cfg.vector_indexes.is_empty() {
                        view! {
                            <EmptyState message="No vector indexes yet. Register the stores you want to target from the ingest pipeline." />
                        }.into_any()
                    } else {
                        cfg.vector_indexes.iter().map(|index| {
                            let vs_provider_name = vector_store_provider_name_for(&cfg.vector_store_providers, index.vector_store_provider_id);
                            let index_for_edit = index.clone();
                            let index_for_delete = index.clone();
                            view! {
                                <div class="card-outer p-4 flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                                    <div class="space-y-2">
                                        <h3 class="text-lg font-semibold break-all">{index.name.clone()}</h3>
                                        <div class="flex gap-2 flex-wrap">
                                            <MetaPill label=vs_provider_name />
                                            <MetaPill label=format!("{} dims", index.dimensions) />
                                            <MetaPill label=short_uuid(index.index_id) />
                                        </div>
                                    </div>
                                    <div class="flex gap-2 flex-wrap">
                                        <button
                                            class="btn"
                                            disabled=busy
                                            on:click=move |_| on_edit.with_value(|f| f(index_for_edit.clone()))
                                        >
                                            "EDIT"
                                        </button>
                                        <button
                                            class="btn"
                                            disabled=busy
                                            on:click=move |_| set_delete_dialog.set(Some(DeleteDialog::VectorIndex(index_for_delete.clone())))
                                        >
                                            "DELETE"
                                        </button>
                                    </div>
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                })}
            </div>
        </section>
    }
}

#[component]
pub fn PipelineConfigurationsPanel(
    config: StoredValue<ConfigurationDto>,
    pipeline_configurations: StoredValue<Vec<PipelineConfigurationDto>>,
    busy: ReadSignal<bool>,
    on_add: Box<dyn Fn(leptos::ev::MouseEvent) + Send + Sync>,
    on_edit: Box<dyn Fn(PipelineConfigurationDto) + Send + Sync>,
    set_delete_pipeline_dialog: WriteSignal<Option<PipelineConfigurationDto>>,
) -> impl IntoView {
    let on_edit = StoredValue::new(on_edit);
    view! {
        <section class="space-y-4">
            <PanelHeader
                title="PIPELINE_CONFIGURATIONS"
                description="Named pipeline configurations tie together an embedding model, a generation model, and a vector index for a specific environment."
                action_label="ADD_PIPELINE_CONFIGURATION"
                action_disabled=move || {
                    busy.get()
                        || config.with_value(|cfg| cfg.embedding_models.is_empty() || cfg.generation_models.is_empty() || cfg.vector_indexes.is_empty())
                }
                on_action=Box::new(on_add)
            />
            <div class="space-y-3">
                {pipeline_configurations.with_value(|pcs| {
                    if pcs.is_empty() {
                        view! {
                            <EmptyState message="No pipeline configurations yet. Add providers, models, and a vector index first, then create a named configuration." />
                        }.into_any()
                    } else {
                        pcs.iter().map(|pc| {
                            let pc_for_edit = pc.clone();
                            let pc_for_delete = pc.clone();
                            let embedding_label = pc.embedding_model_name.clone()
                                .unwrap_or_else(|| short_uuid(pc.embedding_model_id));
                            let generation_label = pc.generation_model_name.clone()
                                .unwrap_or_else(|| short_uuid(pc.generation_model_id));
                            let index_label = pc.vector_index_name.clone()
                                .unwrap_or_else(|| short_uuid(pc.vector_index_id));
                            view! {
                                <div class="card-outer p-4 flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                                    <div class="space-y-2">
                                        <h3 class="text-lg font-semibold">{pc.name.clone()}</h3>
                                        <div class="flex gap-2 flex-wrap">
                                            <MetaPill label=format!("embed: {embedding_label}") />
                                            <MetaPill label=format!("gen: {generation_label}") />
                                            <MetaPill label=format!("index: {index_label}") />
                                            <MetaPill label=short_uuid(pc.pipeline_configuration_id) />
                                        </div>
                                    </div>
                                    <div class="flex gap-2 flex-wrap">
                                        <button
                                            class="btn"
                                            disabled=busy
                                            on:click=move |_| on_edit.with_value(|f| f(pc_for_edit.clone()))
                                        >
                                            "EDIT"
                                        </button>
                                        <button
                                            class="btn"
                                            disabled=busy
                                            on:click=move |_| set_delete_pipeline_dialog.set(Some(pc_for_delete.clone()))
                                        >
                                            "DELETE"
                                        </button>
                                    </div>
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                })}
            </div>
        </section>
    }
}
