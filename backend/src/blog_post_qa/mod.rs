use worker::{DurableObject, Env, Request, Response, Result, SqlStorage, State, durable_object};

#[durable_object]
pub struct BlogPostQA {
    state: State,
}

impl BlogPostQA {
    fn sql(&self) -> SqlStorage {
        self.state.storage().sql()
    }
}

impl DurableObject for BlogPostQA {
    fn new(state: State, _env: Env) -> Self {
        Self { state }
    }

    async fn fetch(&self, _req: Request) -> Result<Response> {
        let _ = self.sql();
        Response::error("Not Found", 404)
    }

    async fn alarm(&self) -> Result<Response> {
        Response::empty()
    }
}
