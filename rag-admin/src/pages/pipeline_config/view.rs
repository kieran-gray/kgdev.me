use leptos::prelude::*;
use uuid::Uuid;

use crate::shared::{
    AddEmbeddingModelDto, AddGenerationModelDto, AddProviderDto, AddVectorIndexDto,
    AiProviderDto, ConfigurationCommandDto, EmbeddingModelDto, GenerationModelDto,
    PipelineConfigurationDto, ProviderType, RemoveAiProviderDto, RemoveEmbeddingModelDto,
    RemoveGenerationModelDto, RemoveVectorIndexDto, RemoveVectorStoreProviderDto,
    SetCurrentEmbeddingModelDto, SetCurrentGenerationModelDto, SetCurrentVectorIndexDto,
    UpdateAiProviderDto, UpdateEmbeddingModelDto, UpdateGenerationModelDto, UpdateVectorIndexDto,
    UpdateVectorStoreProviderDto, VectorIndexDto, VectorStoreProviderDto,
};

use super::commands::{
    default_provider_id, default_vector_store_provider_id, optional_name, parse_uuid_or_none,
    provider_name_for, run_configuration_command, vector_store_provider_name_for, ConfigTab,
};
use super::components::{
    CurrentStepCard, DialogActions, DialogShell, DialogStatus, Field, TabButton,
};
use super::dialogs::{delete_dialog_label, AddDialog, DeleteDialog, EditDialog};
use super::panels::{EmbeddingModelsPanel, GenerationModelsPanel, ProvidersPanel, VectorIndexesPanel};

#[component]
pub fn NewSettingsView(
    config: PipelineConfigurationDto,
    active_tab: ReadSignal<ConfigTab>,
    set_active_tab: WriteSignal<ConfigTab>,
    busy: ReadSignal<bool>,
    set_busy: WriteSignal<bool>,
    set_status: WriteSignal<Option<(bool, String)>>,
    set_refresh: WriteSignal<u32>,
) -> impl IntoView {
    let config = StoredValue::new(config);
    let (add_dialog, set_add_dialog) = signal::<Option<AddDialog>>(None);
    let (edit_dialog, set_edit_dialog) = signal::<Option<EditDialog>>(None);
    let (delete_dialog, set_delete_dialog) = signal::<Option<DeleteDialog>>(None);
    let (dialog_status, set_dialog_status) = signal::<Option<String>>(None);

    let (provider_name, set_provider_name) = signal(String::new());
    let (provider_type, set_provider_type) = signal(ProviderType::Ai);

    let (embedding_provider_id, set_embedding_provider_id) =
        signal(default_provider_id(&config.get_value().ai_providers));
    let (embedding_model, set_embedding_model) = signal(String::new());
    let (embedding_dimensions, set_embedding_dimensions) = signal(
        config
            .get_value()
            .current_vector_index
            .map(|v| v.dimensions)
            .unwrap_or(1024),
    );
    let (generation_provider_id, set_generation_provider_id) =
        signal(default_provider_id(&config.get_value().ai_providers));
    let (generation_model, set_generation_model) = signal(String::new());
    let (vector_store_provider_id, set_vector_store_provider_id) = signal(
        default_vector_store_provider_id(&config.get_value().vector_store_providers),
    );
    let (vector_index_name, set_vector_index_name) = signal(String::new());
    let (vector_index_dimensions, set_vector_index_dimensions) = signal(
        config
            .get_value()
            .current_embedding_model
            .map(|m| m.dimensions)
            .unwrap_or(1024),
    );

    let open_add_provider = move |_| {
        set_dialog_status.set(None);
        set_provider_name.set(String::new());
        set_provider_type.set(ProviderType::Ai);
        set_add_dialog.set(Some(AddDialog::Provider));
    };
    let open_add_embedding = move |_| {
        set_dialog_status.set(None);
        set_embedding_provider_id.set(default_provider_id(&config.get_value().ai_providers));
        set_embedding_model.set(String::new());
        set_embedding_dimensions.set(
            config
                .get_value()
                .current_vector_index
                .map(|i| i.dimensions)
                .unwrap_or(1024),
        );
        set_add_dialog.set(Some(AddDialog::EmbeddingModel));
    };
    let open_add_generation = move |_| {
        set_dialog_status.set(None);
        set_generation_provider_id.set(default_provider_id(&config.get_value().ai_providers));
        set_generation_model.set(String::new());
        set_add_dialog.set(Some(AddDialog::GenerationModel));
    };
    let open_add_vector_index = move |_| {
        set_dialog_status.set(None);
        set_vector_store_provider_id.set(default_vector_store_provider_id(
            &config.get_value().vector_store_providers,
        ));
        set_vector_index_name.set(String::new());
        set_vector_index_dimensions.set(
            config
                .get_value()
                .current_embedding_model
                .map(|m| m.dimensions)
                .unwrap_or(1024),
        );
        set_add_dialog.set(Some(AddDialog::VectorIndex));
    };
    let open_edit_ai_provider = move |provider: AiProviderDto| {
        set_dialog_status.set(None);
        set_provider_name.set(provider.name.clone());
        set_edit_dialog.set(Some(EditDialog::AiProvider(provider)));
    };
    let open_edit_vs_provider = move |provider: VectorStoreProviderDto| {
        set_dialog_status.set(None);
        set_provider_name.set(provider.name.clone());
        set_edit_dialog.set(Some(EditDialog::VectorStoreProvider(provider)));
    };
    let open_edit_embedding = move |model: EmbeddingModelDto| {
        set_dialog_status.set(None);
        set_embedding_provider_id.set(Some(model.provider_id));
        set_embedding_model.set(model.model.clone());
        set_embedding_dimensions.set(model.dimensions);
        set_edit_dialog.set(Some(EditDialog::EmbeddingModel(model)));
    };
    let open_edit_generation = move |model: GenerationModelDto| {
        set_dialog_status.set(None);
        set_generation_provider_id.set(Some(model.provider_id));
        set_generation_model.set(model.model.clone());
        set_edit_dialog.set(Some(EditDialog::GenerationModel(model)));
    };
    let open_edit_vector_index = move |index: VectorIndexDto| {
        set_dialog_status.set(None);
        set_vector_store_provider_id.set(Some(index.vector_store_provider_id));
        set_vector_index_name.set(index.name.clone());
        set_vector_index_dimensions.set(index.dimensions);
        set_edit_dialog.set(Some(EditDialog::VectorIndex(index)));
    };

    let submit_add_provider = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::AddProvider(AddProviderDto {
                name: provider_name.get_untracked(),
                provider_type: provider_type.get_untracked(),
            }),
            "PROVIDER_ADDED",
            set_busy,
            set_status,
            Some(set_dialog_status),
            set_refresh,
            move || {
                set_dialog_status.set(None);
                set_add_dialog.set(None);
            },
        );
    };
    let submit_add_embedding = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(provider_id) = embedding_provider_id.get_untracked() else {
            set_dialog_status.set(Some("ADD_PROVIDER_BEFORE_CREATING_MODELS".into()));
            return;
        };
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::AddEmbeddingModel(AddEmbeddingModelDto {
                provider_id,
                model: embedding_model.get_untracked(),
                dimensions: embedding_dimensions.get_untracked(),
            }),
            "EMBEDDING_MODEL_ADDED",
            set_busy,
            set_status,
            Some(set_dialog_status),
            set_refresh,
            move || {
                set_dialog_status.set(None);
                set_add_dialog.set(None);
            },
        );
    };
    let submit_add_generation = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(provider_id) = generation_provider_id.get_untracked() else {
            set_dialog_status.set(Some("ADD_PROVIDER_BEFORE_CREATING_MODELS".into()));
            return;
        };
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::AddGenerationModel(AddGenerationModelDto {
                provider_id,
                model: generation_model.get_untracked(),
            }),
            "GENERATION_MODEL_ADDED",
            set_busy,
            set_status,
            Some(set_dialog_status),
            set_refresh,
            move || {
                set_dialog_status.set(None);
                set_add_dialog.set(None);
            },
        );
    };
    let submit_add_vector_index = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(provider_id) = vector_store_provider_id.get_untracked() else {
            set_dialog_status.set(Some("ADD_VECTOR_STORE_PROVIDER_BEFORE_CREATING_INDEX".into()));
            return;
        };
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::AddVectorIndex(AddVectorIndexDto {
                vector_store_provider_id: provider_id,
                name: vector_index_name.get_untracked(),
                dimensions: vector_index_dimensions.get_untracked(),
            }),
            "VECTOR_INDEX_ADDED",
            set_busy,
            set_status,
            Some(set_dialog_status),
            set_refresh,
            move || {
                set_dialog_status.set(None);
                set_add_dialog.set(None);
            },
        );
    };
    let submit_edit_ai_provider = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(EditDialog::AiProvider(provider)) = edit_dialog.get_untracked() else {
            return;
        };
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::UpdateAiProvider(UpdateAiProviderDto {
                provider_id: provider.provider_id,
                name: provider_name.get_untracked(),
            }),
            "AI_PROVIDER_UPDATED",
            set_busy,
            set_status,
            Some(set_dialog_status),
            set_refresh,
            move || {
                set_dialog_status.set(None);
                set_edit_dialog.set(None);
            },
        );
    };
    let submit_edit_vs_provider = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(EditDialog::VectorStoreProvider(provider)) = edit_dialog.get_untracked() else {
            return;
        };
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::UpdateVectorStoreProvider(UpdateVectorStoreProviderDto {
                provider_id: provider.provider_id,
                name: provider_name.get_untracked(),
            }),
            "VECTOR_STORE_PROVIDER_UPDATED",
            set_busy,
            set_status,
            Some(set_dialog_status),
            set_refresh,
            move || {
                set_dialog_status.set(None);
                set_edit_dialog.set(None);
            },
        );
    };
    let submit_edit_embedding = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(EditDialog::EmbeddingModel(model)) = edit_dialog.get_untracked() else {
            return;
        };
        let Some(provider_id) = embedding_provider_id.get_untracked() else {
            set_dialog_status.set(Some("SELECT_PROVIDER_FOR_EMBEDDING_MODEL".into()));
            return;
        };
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::UpdateEmbeddingModel(UpdateEmbeddingModelDto {
                model_id: model.embedding_model_id,
                provider_id,
                model: embedding_model.get_untracked(),
                dimensions: embedding_dimensions.get_untracked(),
            }),
            "EMBEDDING_MODEL_UPDATED",
            set_busy,
            set_status,
            Some(set_dialog_status),
            set_refresh,
            move || {
                set_dialog_status.set(None);
                set_edit_dialog.set(None);
            },
        );
    };
    let submit_edit_generation = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(EditDialog::GenerationModel(model)) = edit_dialog.get_untracked() else {
            return;
        };
        let Some(provider_id) = generation_provider_id.get_untracked() else {
            set_dialog_status.set(Some("SELECT_PROVIDER_FOR_GENERATION_MODEL".into()));
            return;
        };
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::UpdateGenerationModel(UpdateGenerationModelDto {
                model_id: model.generation_model_id,
                provider_id,
                model: generation_model.get_untracked(),
            }),
            "GENERATION_MODEL_UPDATED",
            set_busy,
            set_status,
            Some(set_dialog_status),
            set_refresh,
            move || {
                set_dialog_status.set(None);
                set_edit_dialog.set(None);
            },
        );
    };
    let submit_edit_vector_index = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(EditDialog::VectorIndex(index)) = edit_dialog.get_untracked() else {
            return;
        };
        let Some(provider_id) = vector_store_provider_id.get_untracked() else {
            set_dialog_status.set(Some("ADD_VECTOR_STORE_PROVIDER_BEFORE_CREATING_INDEX".into()));
            return;
        };
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::UpdateVectorIndex(UpdateVectorIndexDto {
                index_id: index.index_id,
                vector_store_provider_id: provider_id,
                name: vector_index_name.get_untracked(),
                dimensions: vector_index_dimensions.get_untracked(),
            }),
            "VECTOR_INDEX_UPDATED",
            set_busy,
            set_status,
            Some(set_dialog_status),
            set_refresh,
            move || {
                set_dialog_status.set(None);
                set_edit_dialog.set(None);
            },
        );
    };

    let confirm_delete = move |_| {
        if busy.get_untracked() {
            return;
        }
        let Some(dialog) = delete_dialog.get_untracked() else {
            return;
        };
        match dialog {
            DeleteDialog::AiProvider(provider) => run_configuration_command(
                ConfigurationCommandDto::RemoveAiProvider(RemoveAiProviderDto {
                    provider_id: provider.provider_id,
                }),
                "AI_PROVIDER_REMOVED",
                set_busy,
                set_status,
                Some(set_dialog_status),
                set_refresh,
                move || {
                    set_dialog_status.set(None);
                    set_delete_dialog.set(None);
                },
            ),
            DeleteDialog::VectorStoreProvider(provider) => run_configuration_command(
                ConfigurationCommandDto::RemoveVectorStoreProvider(RemoveVectorStoreProviderDto {
                    provider_id: provider.provider_id,
                }),
                "VECTOR_STORE_PROVIDER_REMOVED",
                set_busy,
                set_status,
                Some(set_dialog_status),
                set_refresh,
                move || {
                    set_dialog_status.set(None);
                    set_delete_dialog.set(None);
                },
            ),
            DeleteDialog::EmbeddingModel(model) => run_configuration_command(
                ConfigurationCommandDto::RemoveEmbeddingModel(RemoveEmbeddingModelDto {
                    model_id: model.embedding_model_id,
                }),
                "EMBEDDING_MODEL_REMOVED",
                set_busy,
                set_status,
                Some(set_dialog_status),
                set_refresh,
                move || {
                    set_dialog_status.set(None);
                    set_delete_dialog.set(None);
                },
            ),
            DeleteDialog::GenerationModel(model) => run_configuration_command(
                ConfigurationCommandDto::RemoveGenerationModel(RemoveGenerationModelDto {
                    model_id: model.generation_model_id,
                }),
                "GENERATION_MODEL_REMOVED",
                set_busy,
                set_status,
                Some(set_dialog_status),
                set_refresh,
                move || {
                    set_dialog_status.set(None);
                    set_delete_dialog.set(None);
                },
            ),
            DeleteDialog::VectorIndex(index) => run_configuration_command(
                ConfigurationCommandDto::RemoveVectorIndex(RemoveVectorIndexDto {
                    index_id: index.index_id,
                }),
                "VECTOR_INDEX_REMOVED",
                set_busy,
                set_status,
                Some(set_dialog_status),
                set_refresh,
                move || {
                    set_dialog_status.set(None);
                    set_delete_dialog.set(None);
                },
            ),
        }
    };

    let set_current_embedding = move |model_id: Uuid| {
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::SetCurrentEmbeddingModel(SetCurrentEmbeddingModelDto {
                model_id,
            }),
            "CURRENT_EMBEDDING_MODEL_UPDATED",
            set_busy,
            set_status,
            None,
            set_refresh,
            || {},
        );
    };
    let set_current_generation = move |model_id: Uuid| {
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::SetCurrentGenerationModel(SetCurrentGenerationModelDto {
                model_id,
            }),
            "CURRENT_GENERATION_MODEL_UPDATED",
            set_busy,
            set_status,
            None,
            set_refresh,
            || {},
        );
    };
    let set_current_vector_index = move |index_id: Uuid| {
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::SetCurrentVectorIndex(SetCurrentVectorIndexDto { index_id }),
            "CURRENT_VECTOR_INDEX_UPDATED",
            set_busy,
            set_status,
            None,
            set_refresh,
            || {},
        );
    };

    view! {
        <Show when=move || add_dialog.get().is_some()>
            <DialogShell
                title=move || match add_dialog.get() {
                    Some(AddDialog::Provider) => "ADD_PROVIDER",
                    Some(AddDialog::EmbeddingModel) => "ADD_EMBEDDING_MODEL",
                    Some(AddDialog::GenerationModel) => "ADD_GENERATION_MODEL",
                    Some(AddDialog::VectorIndex) => "ADD_VECTOR_INDEX",
                    None => "",
                }
                subtitle=move || match add_dialog.get() {
                    Some(AddDialog::Provider) => "Select the type then give the provider a short, stable name.",
                    Some(AddDialog::EmbeddingModel) => "Attach the embedding model to a provider and keep dimensions aligned with the index you intend to use.",
                    Some(AddDialog::GenerationModel) => "Generation models stay lean: provider plus model id.",
                    Some(AddDialog::VectorIndex) => "Index dimensions should match the embedding model that writes into it.",
                    None => "",
                }
                busy=busy
                on_close=Box::new(move || { set_dialog_status.set(None); set_add_dialog.set(None); })
            >
                {move || match add_dialog.get() {
                    Some(AddDialog::Provider) => view! {
                        <form class="space-y-4" on:submit=submit_add_provider>
                            <DialogStatus message=dialog_status />
                            <Field label="PROVIDER_TYPE" hint="AI for models, VECTOR_STORE for indexes">
                                <div class="flex gap-1">
                                    <button
                                        type="button"
                                        class=move || if provider_type.get() == ProviderType::Ai { "btn btn-primary" } else { "btn" }
                                        on:click=move |_| set_provider_type.set(ProviderType::Ai)
                                    >
                                        "AI"
                                    </button>
                                    <button
                                        type="button"
                                        class=move || if provider_type.get() == ProviderType::VectorStore { "btn btn-primary" } else { "btn" }
                                        on:click=move |_| set_provider_type.set(ProviderType::VectorStore)
                                    >
                                        "VECTOR_STORE"
                                    </button>
                                </div>
                            </Field>
                            <Field label="PROVIDER_NAME" hint="e.g. OpenAI, Voyage, Qdrant, Pinecone">
                                <input class="input font-mono text-sm" prop:value=provider_name on:input=move |e| set_provider_name.set(event_target_value(&e)) />
                            </Field>
                            <DialogActions busy=busy submit_label="CREATE_PROVIDER" cancel=Box::new(move || { set_dialog_status.set(None); set_add_dialog.set(None); }) />
                        </form>
                    }.into_any(),
                    Some(AddDialog::EmbeddingModel) => view! {
                        <form class="space-y-4" on:submit=submit_add_embedding>
                            <DialogStatus message=dialog_status />
                            <Field label="PROVIDER" hint="who owns this model">
                                <select class="input font-mono text-sm"
                                    prop:value=move || embedding_provider_id.get().map(|id| id.to_string()).unwrap_or_default()
                                    on:change=move |e| set_embedding_provider_id.set(parse_uuid_or_none(&event_target_value(&e)))
                                >
                                    <option value="">"-- select provider --"</option>
                                    {config.with_value(|cfg| cfg.ai_providers.iter().map(|p| view! { <option value=p.provider_id.to_string()>{p.name.clone()}</option> }).collect_view())}
                                </select>
                            </Field>
                            <Field label="MODEL_ID" hint="provider-specific model identifier">
                                <input class="input font-mono text-sm" prop:value=embedding_model on:input=move |e| set_embedding_model.set(event_target_value(&e)) />
                            </Field>
                            <Field label="DIMENSIONS" hint="must match the target vector index">
                                <input class="input font-mono text-sm" type="number" min="1"
                                    prop:value=move || embedding_dimensions.get().to_string()
                                    on:input=move |e| set_embedding_dimensions.set(event_target_value(&e).parse().unwrap_or(0))
                                />
                            </Field>
                            <DialogActions busy=busy submit_label="CREATE_EMBEDDING_MODEL" cancel=Box::new(move || { set_dialog_status.set(None); set_add_dialog.set(None); }) />
                        </form>
                    }.into_any(),
                    Some(AddDialog::GenerationModel) => view! {
                        <form class="space-y-4" on:submit=submit_add_generation>
                            <DialogStatus message=dialog_status />
                            <Field label="PROVIDER" hint="who owns this model">
                                <select class="input font-mono text-sm"
                                    prop:value=move || generation_provider_id.get().map(|id| id.to_string()).unwrap_or_default()
                                    on:change=move |e| set_generation_provider_id.set(parse_uuid_or_none(&event_target_value(&e)))
                                >
                                    <option value="">"-- select provider --"</option>
                                    {config.with_value(|cfg| cfg.ai_providers.iter().map(|p| view! { <option value=p.provider_id.to_string()>{p.name.clone()}</option> }).collect_view())}
                                </select>
                            </Field>
                            <Field label="MODEL_ID" hint="chat/completion model identifier">
                                <input class="input font-mono text-sm" prop:value=generation_model on:input=move |e| set_generation_model.set(event_target_value(&e)) />
                            </Field>
                            <DialogActions busy=busy submit_label="CREATE_GENERATION_MODEL" cancel=Box::new(move || { set_dialog_status.set(None); set_add_dialog.set(None); }) />
                        </form>
                    }.into_any(),
                    Some(AddDialog::VectorIndex) => view! {
                        <form class="space-y-4" on:submit=submit_add_vector_index>
                            <DialogStatus message=dialog_status />
                            <Field label="VECTOR_STORE_PROVIDER" hint="backend that hosts this index">
                                <select class="input font-mono text-sm"
                                    prop:value=move || vector_store_provider_id.get().map(|id| id.to_string()).unwrap_or_default()
                                    on:change=move |e| set_vector_store_provider_id.set(parse_uuid_or_none(&event_target_value(&e)))
                                >
                                    <option value="">"-- select vector store provider --"</option>
                                    {config.with_value(|cfg| cfg.vector_store_providers.iter().map(|p| view! { <option value=p.provider_id.to_string()>{p.name.clone()}</option> }).collect_view())}
                                </select>
                            </Field>
                            <Field label="INDEX_NAME" hint="external vector store identifier">
                                <input class="input font-mono text-sm" prop:value=vector_index_name on:input=move |e| set_vector_index_name.set(event_target_value(&e)) />
                            </Field>
                            <Field label="DIMENSIONS" hint="must match the embedding model output">
                                <input class="input font-mono text-sm" type="number" min="1"
                                    prop:value=move || vector_index_dimensions.get().to_string()
                                    on:input=move |e| set_vector_index_dimensions.set(event_target_value(&e).parse().unwrap_or(0))
                                />
                            </Field>
                            <DialogActions busy=busy submit_label="CREATE_VECTOR_INDEX" cancel=Box::new(move || { set_dialog_status.set(None); set_add_dialog.set(None); }) />
                        </form>
                    }.into_any(),
                    None => ().into_any(),
                }}
            </DialogShell>
        </Show>

        <Show when=move || edit_dialog.get().is_some()>
            <DialogShell
                title=move || match edit_dialog.get() {
                    Some(EditDialog::AiProvider(_)) => "EDIT_AI_PROVIDER",
                    Some(EditDialog::VectorStoreProvider(_)) => "EDIT_VECTOR_STORE_PROVIDER",
                    Some(EditDialog::EmbeddingModel(_)) => "EDIT_EMBEDDING_MODEL",
                    Some(EditDialog::GenerationModel(_)) => "EDIT_GENERATION_MODEL",
                    Some(EditDialog::VectorIndex(_)) => "EDIT_VECTOR_INDEX",
                    None => "",
                }
                subtitle=move || match edit_dialog.get() {
                    Some(EditDialog::AiProvider(_)) => "Provider names should stay short, stable, and recognizable across related models.",
                    Some(EditDialog::VectorStoreProvider(_)) => "Rename the backend system. The name is a label only; no connection details are stored here.",
                    Some(EditDialog::EmbeddingModel(_)) => "You can move the model to another provider here if ownership has changed.",
                    Some(EditDialog::GenerationModel(_)) => "Generation models can be reassigned to a different provider without recreating the record.",
                    Some(EditDialog::VectorIndex(_)) => "Use edits for renames, corrected dimensions, or moving the index to a different backend.",
                    None => "",
                }
                busy=busy
                on_close=Box::new(move || { set_dialog_status.set(None); set_edit_dialog.set(None); })
            >
                {move || match edit_dialog.get() {
                    Some(EditDialog::AiProvider(_)) => view! {
                        <form class="space-y-4" on:submit=submit_edit_ai_provider>
                            <DialogStatus message=dialog_status />
                            <Field label="PROVIDER_NAME" hint="short recognizable label">
                                <input class="input font-mono text-sm" prop:value=provider_name on:input=move |e| set_provider_name.set(event_target_value(&e)) />
                            </Field>
                            <DialogActions busy=busy submit_label="SAVE_PROVIDER" cancel=Box::new(move || { set_dialog_status.set(None); set_edit_dialog.set(None); }) />
                        </form>
                    }.into_any(),
                    Some(EditDialog::VectorStoreProvider(_)) => view! {
                        <form class="space-y-4" on:submit=submit_edit_vs_provider>
                            <DialogStatus message=dialog_status />
                            <Field label="PROVIDER_NAME" hint="e.g. Qdrant, Pinecone, pgvector">
                                <input class="input font-mono text-sm" prop:value=provider_name on:input=move |e| set_provider_name.set(event_target_value(&e)) />
                            </Field>
                            <DialogActions busy=busy submit_label="SAVE_VECTOR_STORE_PROVIDER" cancel=Box::new(move || { set_dialog_status.set(None); set_edit_dialog.set(None); }) />
                        </form>
                    }.into_any(),
                    Some(EditDialog::EmbeddingModel(_)) => view! {
                        <form class="space-y-4" on:submit=submit_edit_embedding>
                            <DialogStatus message=dialog_status />
                            <Field label="PROVIDER" hint="who owns this model">
                                <select class="input font-mono text-sm"
                                    prop:value=move || embedding_provider_id.get().map(|id| id.to_string()).unwrap_or_default()
                                    on:change=move |e| set_embedding_provider_id.set(parse_uuid_or_none(&event_target_value(&e)))
                                >
                                    <option value="">"-- select provider --"</option>
                                    {config.with_value(|cfg| cfg.ai_providers.iter().map(|p| view! { <option value=p.provider_id.to_string()>{p.name.clone()}</option> }).collect_view())}
                                </select>
                            </Field>
                            <Field label="MODEL_ID" hint="provider-specific model identifier">
                                <input class="input font-mono text-sm" prop:value=embedding_model on:input=move |e| set_embedding_model.set(event_target_value(&e)) />
                            </Field>
                            <Field label="DIMENSIONS" hint="must match the target vector index">
                                <input class="input font-mono text-sm" type="number" min="1"
                                    prop:value=move || embedding_dimensions.get().to_string()
                                    on:input=move |e| set_embedding_dimensions.set(event_target_value(&e).parse().unwrap_or(0))
                                />
                            </Field>
                            <DialogActions busy=busy submit_label="SAVE_EMBEDDING_MODEL" cancel=Box::new(move || { set_dialog_status.set(None); set_edit_dialog.set(None); }) />
                        </form>
                    }.into_any(),
                    Some(EditDialog::GenerationModel(_)) => view! {
                        <form class="space-y-4" on:submit=submit_edit_generation>
                            <DialogStatus message=dialog_status />
                            <Field label="PROVIDER" hint="who owns this model">
                                <select class="input font-mono text-sm"
                                    prop:value=move || generation_provider_id.get().map(|id| id.to_string()).unwrap_or_default()
                                    on:change=move |e| set_generation_provider_id.set(parse_uuid_or_none(&event_target_value(&e)))
                                >
                                    <option value="">"-- select provider --"</option>
                                    {config.with_value(|cfg| cfg.ai_providers.iter().map(|p| view! { <option value=p.provider_id.to_string()>{p.name.clone()}</option> }).collect_view())}
                                </select>
                            </Field>
                            <Field label="MODEL_ID" hint="chat/completion model identifier">
                                <input class="input font-mono text-sm" prop:value=generation_model on:input=move |e| set_generation_model.set(event_target_value(&e)) />
                            </Field>
                            <DialogActions busy=busy submit_label="SAVE_GENERATION_MODEL" cancel=Box::new(move || { set_dialog_status.set(None); set_edit_dialog.set(None); }) />
                        </form>
                    }.into_any(),
                    Some(EditDialog::VectorIndex(_)) => view! {
                        <form class="space-y-4" on:submit=submit_edit_vector_index>
                            <DialogStatus message=dialog_status />
                            <Field label="VECTOR_STORE_PROVIDER" hint="backend that hosts this index">
                                <select class="input font-mono text-sm"
                                    prop:value=move || vector_store_provider_id.get().map(|id| id.to_string()).unwrap_or_default()
                                    on:change=move |e| set_vector_store_provider_id.set(parse_uuid_or_none(&event_target_value(&e)))
                                >
                                    <option value="">"-- select vector store provider --"</option>
                                    {config.with_value(|cfg| cfg.vector_store_providers.iter().map(|p| view! { <option value=p.provider_id.to_string()>{p.name.clone()}</option> }).collect_view())}
                                </select>
                            </Field>
                            <Field label="INDEX_NAME" hint="external vector store identifier">
                                <input class="input font-mono text-sm" prop:value=vector_index_name on:input=move |e| set_vector_index_name.set(event_target_value(&e)) />
                            </Field>
                            <Field label="DIMENSIONS" hint="must match the embedding model output">
                                <input class="input font-mono text-sm" type="number" min="1"
                                    prop:value=move || vector_index_dimensions.get().to_string()
                                    on:input=move |e| set_vector_index_dimensions.set(event_target_value(&e).parse().unwrap_or(0))
                                />
                            </Field>
                            <DialogActions busy=busy submit_label="SAVE_VECTOR_INDEX" cancel=Box::new(move || { set_dialog_status.set(None); set_edit_dialog.set(None); }) />
                        </form>
                    }.into_any(),
                    None => ().into_any(),
                }}
            </DialogShell>
        </Show>

        <Show when=move || delete_dialog.get().is_some()>
            <DialogShell
                title=move || "CONFIRM_DELETE"
                subtitle=move || "Deletes are explicit because downstream models and active selections may still depend on the record."
                busy=busy
                on_close=Box::new(move || { set_dialog_status.set(None); set_delete_dialog.set(None); })
            >
                <div class="space-y-4">
                    <DialogStatus message=dialog_status />
                    <div class="card-outer p-4 bg-black/20">
                        <span class="tech-label opacity-70">
                            {move || delete_dialog_label(delete_dialog.get())}
                        </span>
                    </div>
                    <div class="flex justify-end gap-2">
                        <button class="btn" disabled=busy on:click=move |_| { set_dialog_status.set(None); set_delete_dialog.set(None); }>
                            "CANCEL"
                        </button>
                        <button class="btn btn-primary" disabled=busy on:click=confirm_delete>
                            {move || if busy.get() { "DELETING..." } else { "CONFIRM_DELETE" }}
                        </button>
                    </div>
                </div>
            </DialogShell>
        </Show>

        <div class="grid grid-cols-3 lg:grid-cols-5 gap-px bg-[var(--color-border)] border-y border-x border-[var(--color-border)]">
            <div class="bg-[var(--color-page-bg)] px-6 py-4 flex flex-col">
                <span class="tech-label opacity-40 text-[9px] mb-1">"AI_PROVIDERS"</span>
                <span class="font-mono text-sm font-bold">{config.with_value(|cfg| cfg.ai_providers.len().to_string())}</span>
            </div>
            <div class="bg-[var(--color-page-bg)] px-4 py-4 flex flex-col">
                <span class="tech-label opacity-40 text-[9px] mb-1">"VS_PROVIDERS"</span>
                <span class="font-mono text-sm font-bold">{config.with_value(|cfg| cfg.vector_store_providers.len().to_string())}</span>
            </div>
            <div class="bg-[var(--color-page-bg)] px-4 py-4 flex flex-col">
                <span class="tech-label opacity-40 text-[9px] mb-1">"EMBEDDING"</span>
                <span class="font-mono text-sm font-bold">{config.with_value(|cfg| cfg.embedding_models.len().to_string())}</span>
            </div>
            <div class="bg-[var(--color-page-bg)] px-4 py-4 flex flex-col">
                <span class="tech-label opacity-40 text-[9px] mb-1">"GENERATION"</span>
                <span class="font-mono text-sm font-bold">{config.with_value(|cfg| cfg.generation_models.len().to_string())}</span>
            </div>
            <div class="bg-[var(--color-page-bg)] px-6 py-4 flex flex-col">
                <span class="tech-label opacity-40 text-[9px] mb-1">"INDEXES"</span>
                <span class="font-mono text-sm font-bold">{config.with_value(|cfg| cfg.vector_indexes.len().to_string())}</span>
            </div>
        </div>

        <div class="border-y border-[var(--color-border)] py-8 bg-black/5">
            <div class="px-6 space-y-4">
                <div class="space-y-1">
                    <div class="tech-label opacity-60">"ACTIVE_PIPELINE"</div>
                    <p class="tech-label opacity-50 max-w-3xl">
                        "Selections are model-first. Provider usage is shown on the embedding and generation steps so cross-provider setups stay obvious."
                    </p>
                </div>
                <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                    <CurrentStepCard
                        label="EMBEDDING_STEP"
                        value=config.with_value(|cfg| optional_name(cfg.current_embedding_model.as_ref().map(|m| m.model.as_str())))
                        detail=config.with_value(|cfg| {
                            cfg.current_embedding_model.as_ref()
                                .map(|m| format!("provider: {} | {} dims", provider_name_for(&cfg.ai_providers, m.provider_id), m.dimensions))
                                .unwrap_or_else(|| "provider: UNSET".into())
                        })
                    />
                    <CurrentStepCard
                        label="GENERATION_STEP"
                        value=config.with_value(|cfg| optional_name(cfg.current_generation_model.as_ref().map(|m| m.model.as_str())))
                        detail=config.with_value(|cfg| {
                            cfg.current_generation_model.as_ref()
                                .map(|m| format!("provider: {}", provider_name_for(&cfg.ai_providers, m.provider_id)))
                                .unwrap_or_else(|| "provider: UNSET".into())
                        })
                    />
                    <CurrentStepCard
                        label="VECTOR_INDEX_STEP"
                        value=config.with_value(|cfg| optional_name(cfg.current_vector_index.as_ref().map(|i| i.name.as_str())))
                        detail=config.with_value(|cfg| {
                            cfg.current_vector_index.as_ref()
                                .map(|i| format!("backend: {} | {} dims",
                                    vector_store_provider_name_for(&cfg.vector_store_providers, i.vector_store_provider_id),
                                    i.dimensions))
                                .unwrap_or_else(|| "backend: UNSET".into())
                        })
                    />
                </div>
            </div>
        </div>

        <div class="border-b border-[var(--color-border)] mt-8">
            <div class="px-6 flex gap-1 overflow-x-auto">
                <TabButton label="PROVIDERS" active=move || active_tab.get() == ConfigTab::Providers on_click=Box::new(move || set_active_tab.set(ConfigTab::Providers)) />
                <TabButton label="EMBEDDING_MODELS" active=move || active_tab.get() == ConfigTab::EmbeddingModels on_click=Box::new(move || set_active_tab.set(ConfigTab::EmbeddingModels)) />
                <TabButton label="GENERATION_MODELS" active=move || active_tab.get() == ConfigTab::GenerationModels on_click=Box::new(move || set_active_tab.set(ConfigTab::GenerationModels)) />
                <TabButton label="VECTOR_INDEXES" active=move || active_tab.get() == ConfigTab::VectorIndexes on_click=Box::new(move || set_active_tab.set(ConfigTab::VectorIndexes)) />
            </div>
        </div>

        <div class="pt-6">
            {move || match active_tab.get() {
                ConfigTab::Providers => view! {
                    <div class="px-6">
                        <ProvidersPanel
                            config=config
                            busy=busy
                            on_add=Box::new(open_add_provider)
                            on_edit_ai=Box::new(open_edit_ai_provider)
                            on_edit_vs=Box::new(open_edit_vs_provider)
                            set_delete_dialog=set_delete_dialog
                        />
                    </div>
                }.into_any(),
                ConfigTab::EmbeddingModels => view! {
                    <div class="px-6">
                        <EmbeddingModelsPanel
                            config=config
                            busy=busy
                            on_add=Box::new(open_add_embedding)
                            on_edit=Box::new(open_edit_embedding)
                            set_delete_dialog=set_delete_dialog
                            on_set_current=Box::new(set_current_embedding)
                        />
                    </div>
                }.into_any(),
                ConfigTab::GenerationModels => view! {
                    <div class="px-6">
                        <GenerationModelsPanel
                            config=config
                            busy=busy
                            on_add=Box::new(open_add_generation)
                            on_edit=Box::new(open_edit_generation)
                            set_delete_dialog=set_delete_dialog
                            on_set_current=Box::new(set_current_generation)
                        />
                    </div>
                }.into_any(),
                ConfigTab::VectorIndexes => view! {
                    <div class="px-6">
                        <VectorIndexesPanel
                            config=config
                            busy=busy
                            on_add=Box::new(open_add_vector_index)
                            on_edit=Box::new(open_edit_vector_index)
                            set_delete_dialog=set_delete_dialog
                            on_set_current=Box::new(set_current_vector_index)
                        />
                    </div>
                }.into_any(),
            }}
        </div>
    }
}
