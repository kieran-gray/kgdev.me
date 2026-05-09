use serde::{de::DeserializeOwned, Serialize};

pub trait Aggregate: Sized + Clone + Serialize + DeserializeOwned {
    type Event: Clone;
    type Command;
    type Error: std::error::Error;

    fn aggregate_id(&self) -> String;

    fn apply(&mut self, event: &Self::Event);

    fn handle_command(
        state: Option<&Self>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error>;

    fn from_events(events: &[Self::Event]) -> Option<Self>;
}
