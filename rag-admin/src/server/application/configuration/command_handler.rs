use std::sync::Arc;

use crate::server::application::AppError;
use crate::server::domain::configuration::{
    aggregate::Configuration, commands::ConfigurationCommand,
};
use crate::server::event_sourcing::CommandProcessor;
use crate::shared::ConfigurationCommandDto;

pub struct ConfigurationCommandHandler {
    processor: Arc<CommandProcessor<Configuration>>,
}

impl ConfigurationCommandHandler {
    pub fn new(processor: Arc<CommandProcessor<Configuration>>) -> Arc<Self> {
        Arc::new(Self { processor })
    }

    pub async fn handle(&self, command: ConfigurationCommand) -> Result<(), AppError> {
        self.processor
            .handle(Configuration::singleton_id(), command)
            .await?;
        Ok(())
    }

    pub async fn handle_dto(&self, command: ConfigurationCommandDto) -> Result<(), AppError> {
        self.handle(command.into()).await
    }
}
