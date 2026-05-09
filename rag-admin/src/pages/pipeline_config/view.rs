use leptos::prelude::*;

use crate::shared::{
    AddEmbeddingModelDto, AddGenerationModelDto, AddProviderDto, AddVectorIndexDto, AiProviderDto,
    ConfigurationCommandDto, ConfigurationDto, CreatePipelineConfigurationDto,
    DeletePipelineConfigurationDto, EmbeddingModelDto, GenerationModelDto,
    PipelineConfigurationDto, ProviderType, RemoveAiProviderDto, RemoveEmbeddingModelDto,
    RemoveGenerationModelDto, RemoveVectorIndexDto, RemoveVectorStoreProviderDto,
    UpdateAiProviderDto, UpdateEmbeddingModelDto, UpdateGenerationModelDto,
    UpdatePipelineConfigurationDto, UpdateVectorIndexDto, UpdateVectorStoreProviderDto,
    VectorIndexDto, VectorStoreProviderDto,
};

use super::commands::{
    default_provider_id, default_vector_store_provider_id, parse_uuid_or_none, provider_name_for,
    run_configuration_command, vector_store_provider_name_for, ConfigTab,
};
use super::components::{DialogActions, DialogShell, DialogStatus, Field, TabButton};
use super::dialogs::{delete_dialog_label, AddDialog, DeleteDialog, EditDialog};
use super::panels::{
    EmbeddingModelsPanel, GenerationModelsPanel, PipelineConfigurationsPanel, ProvidersPanel,
    VectorIndexesPanel,
};

#[component]
pub fn NewSettingsView(
    config: ConfigurationDto,
    pipeline_configurations: Vec<PipelineConfigurationDto>,
    active_tab: ReadSignal<ConfigTab>,
    set_active_tab: WriteSignal<ConfigTab>,
    busy: ReadSignal<bool>,
    set_busy: WriteSignal<bool>,
    set_status: WriteSignal<Option<(bool, String)>>,
    set_refresh: WriteSignal<u32>,
) -> impl IntoView {
    let config = StoredValue::new(config);
    let pipeline_configurations = StoredValue::new(pipeline_configurations);

    let (add_dialog, set_add_dialog) = signal::<Option<AddDialog>>(None);
    let (edit_dialog, set_edit_dialog) = signal::<Option<EditDialog>>(None);
    let (delete_dialog, set_delete_dialog) = signal::<Option<DeleteDialog>>(None);
    let (delete_pipeline_dialog, set_delete_pipeline_dialog) =
        signal::<Option<PipelineConfigurationDto>>(None);
    let (dialog_status, set_dialog_status) = signal::<Option<String>>(None);

    let (provider_name, set_provider_name) = signal(String::new());
    let (provider_type, set_provider_type) = signal(ProviderType::Ai);
    let (embedding_provider_id, set_embedding_provider_id) =
        signal(default_provider_id(&config.get_value().ai_providers));
    let (embedding_model, set_embedding_model) = signal(String::new());
    let (embedding_dimensions, set_embedding_dimensions) = signal(1024u32);
    let (generation_provider_id, set_generation_provider_id) =
        signal(default_provider_id(&config.get_value().ai_providers));
    let (generation_model, set_generation_model) = signal(String::new());
    let (vector_store_provider_id, set_vector_store_provider_id) = signal(
        default_vector_store_provider_id(&config.get_value().vector_store_providers),
    );
    let (vector_index_name, set_vector_index_name) = signal(String::new());
    let (vector_index_dimensions, set_vector_index_dimensions) = signal(1024u32);

    let (pc_name, set_pc_name) = signal(String::new());
    let (pc_embedding_model_id, set_pc_embedding_model_id) =
        signal(config.with_value(|cfg| cfg.embedding_models.first().map(|m| m.embedding_model_id)));
    let (pc_generation_model_id, set_pc_generation_model_id) = signal(
        config.with_value(|cfg| cfg.generation_models.first().map(|m| m.generation_model_id)),
    );
    let (pc_vector_index_id, set_pc_vector_index_id) =
        signal(config.with_value(|cfg| cfg.vector_indexes.first().map(|i| i.index_id)));

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
        set_embedding_dimensions.set(1024);
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
        set_vector_index_dimensions.set(1024);
        set_add_dialog.set(Some(AddDialog::VectorIndex));
    };
    let open_add_pipeline_config = move |_| {
        set_dialog_status.set(None);
        set_pc_name.set(String::new());
        set_pc_embedding_model_id.set(
            config.with_value(|cfg| cfg.embedding_models.first().map(|m| m.embedding_model_id)),
        );
        set_pc_generation_model_id.set(
            config.with_value(|cfg| cfg.generation_models.first().map(|m| m.generation_model_id)),
        );
        set_pc_vector_index_id
            .set(config.with_value(|cfg| cfg.vector_indexes.first().map(|i| i.index_id)));
        set_add_dialog.set(Some(AddDialog::PipelineConfiguration));
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
    let open_edit_pipeline_config = move |pc: PipelineConfigurationDto| {
        set_dialog_status.set(None);
        set_pc_name.set(pc.name.clone());
        set_pc_embedding_model_id.set(Some(pc.embedding_model_id));
        set_pc_generation_model_id.set(Some(pc.generation_model_id));
        set_pc_vector_index_id.set(Some(pc.vector_index_id));
        set_edit_dialog.set(Some(EditDialog::PipelineConfiguration(pc)));
    };

    // ── Submit handlers ────────────────────────────────────────────────────────

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
            set_dialog_status.set(Some(
                "ADD_VECTOR_STORE_PROVIDER_BEFORE_CREATING_INDEX".into(),
            ));
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
    let submit_add_pipeline_config = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let (Some(emb), Some(gen), Some(idx)) = (
            pc_embedding_model_id.get_untracked(),
            pc_generation_model_id.get_untracked(),
            pc_vector_index_id.get_untracked(),
        ) else {
            set_dialog_status.set(Some("SELECT_ALL_PIPELINE_CONFIGURATION_FIELDS".into()));
            return;
        };
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::CreatePipelineConfiguration(CreatePipelineConfigurationDto {
                name: pc_name.get_untracked(),
                embedding_model_id: emb,
                generation_model_id: gen,
                vector_index_id: idx,
            }),
            "PIPELINE_CONFIGURATION_CREATED",
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
            set_dialog_status.set(Some(
                "ADD_VECTOR_STORE_PROVIDER_BEFORE_CREATING_INDEX".into(),
            ));
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
    let submit_edit_pipeline_config = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(EditDialog::PipelineConfiguration(pc)) = edit_dialog.get_untracked() else {
            return;
        };
        let (Some(emb), Some(gen), Some(idx)) = (
            pc_embedding_model_id.get_untracked(),
            pc_generation_model_id.get_untracked(),
            pc_vector_index_id.get_untracked(),
        ) else {
            set_dialog_status.set(Some("SELECT_ALL_PIPELINE_CONFIGURATION_FIELDS".into()));
            return;
        };
        if busy.get_untracked() {
            return;
        }
        run_configuration_command(
            ConfigurationCommandDto::UpdatePipelineConfiguration(UpdatePipelineConfigurationDto {
                pipeline_configuration_id: pc.pipeline_configuration_id,
                name: pc_name.get_untracked(),
                embedding_model_id: emb,
                generation_model_id: gen,
                vector_index_id: idx,
            }),
            "PIPELINE_CONFIGURATION_UPDATED",
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

    let confirm_delete_pipeline = move |_| {
        if busy.get_untracked() {
            return;
        }
        let Some(pc) = delete_pipeline_dialog.get_untracked() else {
            return;
        };
        run_configuration_command(
            ConfigurationCommandDto::DeletePipelineConfiguration(DeletePipelineConfigurationDto {
                pipeline_configuration_id: pc.pipeline_configuration_id,
            }),
            "PIPELINE_CONFIGURATION_DELETED",
            set_busy,
            set_status,
            None,
            set_refresh,
            move || {
                set_delete_pipeline_dialog.set(None);
            },
        );
    };

    let pc_fields = move || {
        view! {
            <Field label="NAME" hint="short identifier for this environment, e.g. production, staging">
                <input class="input font-mono text-sm" prop:value=pc_name
                    on:input=move |e| set_pc_name.set(event_target_value(&e)) />
            </Field>
            <Field label="EMBEDDING_MODEL" hint="dimensions must match the vector index">
                <select class="input font-mono text-sm"
                    prop:value=move || pc_embedding_model_id.get().map(|id| id.to_string()).unwrap_or_default()
                    on:change=move |e| set_pc_embedding_model_id.set(parse_uuid_or_none(&event_target_value(&e)))>
                    <option value="">"-- select embedding model --"</option>
                    {config.with_value(|cfg| cfg.embedding_models.iter().map(|m| {
                        let label = format!("{} ({}d, {})", m.model, m.dimensions, provider_name_for(&cfg.ai_providers, m.provider_id));
                        view! { <option value=m.embedding_model_id.to_string()>{label}</option> }
                    }).collect_view())}
                </select>
            </Field>
            <Field label="GENERATION_MODEL" hint="chat/completion model for synthesis">
                <select class="input font-mono text-sm"
                    prop:value=move || pc_generation_model_id.get().map(|id| id.to_string()).unwrap_or_default()
                    on:change=move |e| set_pc_generation_model_id.set(parse_uuid_or_none(&event_target_value(&e)))>
                    <option value="">"-- select generation model --"</option>
                    {config.with_value(|cfg| cfg.generation_models.iter().map(|m| {
                        let label = format!("{} ({})", m.model, provider_name_for(&cfg.ai_providers, m.provider_id));
                        view! { <option value=m.generation_model_id.to_string()>{label}</option> }
                    }).collect_view())}
                </select>
            </Field>
            <Field label="VECTOR_INDEX" hint="dimensions must match the embedding model">
                <select class="input font-mono text-sm"
                    prop:value=move || pc_vector_index_id.get().map(|id| id.to_string()).unwrap_or_default()
                    on:change=move |e| set_pc_vector_index_id.set(parse_uuid_or_none(&event_target_value(&e)))>
                    <option value="">"-- select vector index --"</option>
                    {config.with_value(|cfg| cfg.vector_indexes.iter().map(|i| {
                        let label = format!("{} ({}d, {})", i.name, i.dimensions, vector_store_provider_name_for(&cfg.vector_store_providers, i.vector_store_provider_id));
                        view! { <option value=i.index_id.to_string()>{label}</option> }
                    }).collect_view())}
                </select>
            </Field>
        }
    };

    view! {
        <Show when=move || add_dialog.get().is_some()>
            <DialogShell
                title=move || match add_dialog.get() {
                    Some(AddDialog::Provider) => "ADD_PROVIDER",
                    Some(AddDialog::EmbeddingModel) => "ADD_EMBEDDING_MODEL",
                    Some(AddDialog::GenerationModel) => "ADD_GENERATION_MODEL",
                    Some(AddDialog::VectorIndex) => "ADD_VECTOR_INDEX",
                    Some(AddDialog::PipelineConfiguration) => "ADD_PIPELINE_CONFIGURATION",
                    None => "",
                }
                subtitle=move || match add_dialog.get() {
                    Some(AddDialog::Provider) => "Select the type then give the provider a short, stable name.",
                    Some(AddDialog::EmbeddingModel) => "Attach the embedding model to a provider and keep dimensions aligned with the index.",
                    Some(AddDialog::GenerationModel) => "Generation models stay lean: provider plus model id.",
                    Some(AddDialog::VectorIndex) => "Index dimensions should match the embedding model that writes into it.",
                    Some(AddDialog::PipelineConfiguration) => "Tie together an embedding model, generation model, and vector index for a named environment.",
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
                                    <button type="button"
                                        class=move || if provider_type.get() == ProviderType::Ai { "btn btn-primary" } else { "btn" }
                                        on:click=move |_| set_provider_type.set(ProviderType::Ai)>"AI"</button>
                                    <button type="button"
                                        class=move || if provider_type.get() == ProviderType::VectorStore { "btn btn-primary" } else { "btn" }
                                        on:click=move |_| set_provider_type.set(ProviderType::VectorStore)>"VECTOR_STORE"</button>
                                </div>
                            </Field>
                            <Field label="PROVIDER_NAME" hint="e.g. OpenAI, Voyage, Qdrant">
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
                                    on:change=move |e| set_embedding_provider_id.set(parse_uuid_or_none(&event_target_value(&e)))>
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
                                    on:input=move |e| set_embedding_dimensions.set(event_target_value(&e).parse().unwrap_or(0)) />
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
                                    on:change=move |e| set_generation_provider_id.set(parse_uuid_or_none(&event_target_value(&e)))>
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
                                    on:change=move |e| set_vector_store_provider_id.set(parse_uuid_or_none(&event_target_value(&e)))>
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
                                    on:input=move |e| set_vector_index_dimensions.set(event_target_value(&e).parse().unwrap_or(0)) />
                            </Field>
                            <DialogActions busy=busy submit_label="CREATE_VECTOR_INDEX" cancel=Box::new(move || { set_dialog_status.set(None); set_add_dialog.set(None); }) />
                        </form>
                    }.into_any(),
                    Some(AddDialog::PipelineConfiguration) => view! {
                        <form class="space-y-4" on:submit=submit_add_pipeline_config>
                            <DialogStatus message=dialog_status />
                            {pc_fields()}
                            <DialogActions busy=busy submit_label="CREATE_PIPELINE_CONFIGURATION"
                                cancel=Box::new(move || { set_dialog_status.set(None); set_add_dialog.set(None); }) />
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
                    Some(EditDialog::PipelineConfiguration(_)) => "EDIT_PIPELINE_CONFIGURATION",
                    None => "",
                }
                subtitle=move || match edit_dialog.get() {
                    Some(EditDialog::AiProvider(_)) => "Provider names should stay short, stable, and recognizable.",
                    Some(EditDialog::VectorStoreProvider(_)) => "Rename the backend system. The name is a label only.",
                    Some(EditDialog::EmbeddingModel(_)) => "You can move the model to another provider here if ownership has changed.",
                    Some(EditDialog::GenerationModel(_)) => "Generation models can be reassigned to a different provider.",
                    Some(EditDialog::VectorIndex(_)) => "Use edits for renames, corrected dimensions, or moving to a different backend.",
                    Some(EditDialog::PipelineConfiguration(_)) => "Update the model and index selections for this environment. Dimensions will be re-validated.",
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
                                    on:change=move |e| set_embedding_provider_id.set(parse_uuid_or_none(&event_target_value(&e)))>
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
                                    on:input=move |e| set_embedding_dimensions.set(event_target_value(&e).parse().unwrap_or(0)) />
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
                                    on:change=move |e| set_generation_provider_id.set(parse_uuid_or_none(&event_target_value(&e)))>
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
                                    on:change=move |e| set_vector_store_provider_id.set(parse_uuid_or_none(&event_target_value(&e)))>
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
                                    on:input=move |e| set_vector_index_dimensions.set(event_target_value(&e).parse().unwrap_or(0)) />
                            </Field>
                            <DialogActions busy=busy submit_label="SAVE_VECTOR_INDEX" cancel=Box::new(move || { set_dialog_status.set(None); set_edit_dialog.set(None); }) />
                        </form>
                    }.into_any(),
                    Some(EditDialog::PipelineConfiguration(_)) => view! {
                        <form class="space-y-4" on:submit=submit_edit_pipeline_config>
                            <DialogStatus message=dialog_status />
                            {pc_fields()}
                            <DialogActions busy=busy submit_label="SAVE_PIPELINE_CONFIGURATION"
                                cancel=Box::new(move || { set_dialog_status.set(None); set_edit_dialog.set(None); }) />
                        </form>
                    }.into_any(),
                    None => ().into_any(),
                }}
            </DialogShell>
        </Show>

        <Show when=move || delete_dialog.get().is_some()>
            <DialogShell
                title=move || "CONFIRM_DELETE"
                subtitle=move || "Deletes are explicit because downstream models may still reference this record."
                busy=busy
                on_close=Box::new(move || { set_dialog_status.set(None); set_delete_dialog.set(None); })
            >
                <div class="space-y-4">
                    <DialogStatus message=dialog_status />
                    <div class="card-outer p-4 bg-black/20">
                        <span class="tech-label opacity-70">{move || delete_dialog_label(delete_dialog.get())}</span>
                    </div>
                    <div class="flex justify-end gap-2">
                        <button class="btn" disabled=busy on:click=move |_| { set_dialog_status.set(None); set_delete_dialog.set(None); }>"CANCEL"</button>
                        <button class="btn btn-primary" disabled=busy on:click=confirm_delete>
                            {move || if busy.get() { "DELETING..." } else { "CONFIRM_DELETE" }}
                        </button>
                    </div>
                </div>
            </DialogShell>
        </Show>

        <Show when=move || delete_pipeline_dialog.get().is_some()>
            <DialogShell
                title=move || "CONFIRM_DELETE_PIPELINE_CONFIGURATION"
                subtitle=move || "This will remove the pipeline configuration. The catalog entries it references are not affected."
                busy=busy
                on_close=Box::new(move || { set_delete_pipeline_dialog.set(None); })
            >
                <div class="space-y-4">
                    <div class="card-outer p-4 bg-black/20">
                        <span class="tech-label opacity-70">
                            {move || delete_pipeline_dialog.get().map(|pc| pc.name).unwrap_or_default()}
                        </span>
                    </div>
                    <div class="flex justify-end gap-2">
                        <button class="btn" disabled=busy on:click=move |_| set_delete_pipeline_dialog.set(None)>"CANCEL"</button>
                        <button class="btn btn-primary" disabled=busy on:click=confirm_delete_pipeline>
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

        <div class="border-b border-[var(--color-border)] mt-8">
            <div class="px-6 flex gap-1 overflow-x-auto">
                <TabButton label="PIPELINE_CONFIGURATIONS" active=move || active_tab.get() == ConfigTab::PipelineConfigurations on_click=Box::new(move || set_active_tab.set(ConfigTab::PipelineConfigurations)) />
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
                        <ProvidersPanel config=config busy=busy on_add=Box::new(open_add_provider)
                            on_edit_ai=Box::new(open_edit_ai_provider)
                            on_edit_vs=Box::new(open_edit_vs_provider)
                            set_delete_dialog=set_delete_dialog />
                    </div>
                }.into_any(),
                ConfigTab::EmbeddingModels => view! {
                    <div class="px-6">
                        <EmbeddingModelsPanel config=config busy=busy on_add=Box::new(open_add_embedding)
                            on_edit=Box::new(open_edit_embedding)
                            set_delete_dialog=set_delete_dialog />
                    </div>
                }.into_any(),
                ConfigTab::GenerationModels => view! {
                    <div class="px-6">
                        <GenerationModelsPanel config=config busy=busy on_add=Box::new(open_add_generation)
                            on_edit=Box::new(open_edit_generation)
                            set_delete_dialog=set_delete_dialog />
                    </div>
                }.into_any(),
                ConfigTab::VectorIndexes => view! {
                    <div class="px-6">
                        <VectorIndexesPanel config=config busy=busy on_add=Box::new(open_add_vector_index)
                            on_edit=Box::new(open_edit_vector_index)
                            set_delete_dialog=set_delete_dialog />
                    </div>
                }.into_any(),
                ConfigTab::PipelineConfigurations => view! {
                    <div class="px-6">
                        <PipelineConfigurationsPanel
                            config=config
                            pipeline_configurations=pipeline_configurations
                            busy=busy
                            on_add=Box::new(open_add_pipeline_config)
                            on_edit=Box::new(open_edit_pipeline_config)
                            set_delete_pipeline_dialog=set_delete_pipeline_dialog />
                    </div>
                }.into_any(),
            }}
        </div>
    }
}
