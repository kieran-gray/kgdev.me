use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{Route, Router, Routes},
    ParamSegment, StaticSegment,
};

use crate::components::layout::Layout;
use crate::pages::{
    post_detail::PostDetailPage, posts_list::PostsListPage, settings::SettingsPage,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <link rel="stylesheet" id="leptos" href="/pkg/rag_admin.css" />
                <link rel="shortcut icon" type="image/ico" href="/favicon.ico" />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text="RAG Admin" />
        <Router>
            <Layout>
                <Routes fallback=|| view! { <p class="p-8">"Page not found."</p> }>
                    <Route path=StaticSegment("") view=PostsListPage />
                    <Route path=(StaticSegment("posts"), ParamSegment("slug")) view=PostDetailPage />
                    <Route path=StaticSegment("settings") view=SettingsPage />
                </Routes>
            </Layout>
        </Router>
    }
}
