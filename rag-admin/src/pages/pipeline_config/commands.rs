use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use crate::server_functions::configuration::apply_configuration_command;
use crate::shared::{AiProviderDto, ConfigurationCommandDto, VectorStoreProviderDto};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigTab {
    Providers,
    EmbeddingModels,
    GenerationModels,
    VectorIndexes,
}

pub fn default_provider_id(items: &[AiProviderDto]) -> Option<Uuid> {
    items.first().map(|p| p.provider_id)
}

pub fn default_vector_store_provider_id(items: &[VectorStoreProviderDto]) -> Option<Uuid> {
    items.first().map(|p| p.provider_id)
}

pub fn parse_uuid_or_none(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value).ok()
}

pub fn provider_name_for(providers: &[AiProviderDto], provider_id: Uuid) -> String {
    providers
        .iter()
        .find(|p| p.provider_id == provider_id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| format!("unknown:{provider_id}"))
}

pub fn vector_store_provider_name_for(
    providers: &[VectorStoreProviderDto],
    provider_id: Uuid,
) -> String {
    providers
        .iter()
        .find(|p| p.provider_id == provider_id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| format!("unknown:{provider_id}"))
}

pub fn short_uuid(id: Uuid) -> String {
    id.to_string().chars().take(8).collect()
}

pub fn optional_name(value: Option<&str>) -> String {
    value.unwrap_or("UNSET").to_string()
}

pub fn run_configuration_command<F>(
    command: ConfigurationCommandDto,
    success_message: &'static str,
    set_busy: WriteSignal<bool>,
    set_status: WriteSignal<Option<(bool, String)>>,
    dialog_status: Option<WriteSignal<Option<String>>>,
    set_refresh: WriteSignal<u32>,
    on_success: F,
) where
    F: FnOnce() + 'static,
{
    set_busy.set(true);
    set_status.set(None);
    if let Some(ds) = dialog_status {
        ds.set(None);
    }
    spawn_local(async move {
        match apply_configuration_command(command).await {
            Ok(()) => {
                if let Some(ds) = dialog_status {
                    ds.set(None);
                }
                on_success();
                set_status.set(Some((true, success_message.to_string())));
                set_refresh.update(|v| *v += 1);
            }
            Err(e) => {
                let message = format!("COMMAND_FAULT: {e}");
                if let Some(ds) = dialog_status {
                    ds.set(Some(message));
                } else {
                    set_status.set(Some((false, message)));
                }
            }
        }
        set_busy.set(false);
    });
}
