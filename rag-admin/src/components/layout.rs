use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    view! {
        <div class="min-h-screen flex flex-col relative">
            <div class="fixed bottom-2 right-2 tech-label pointer-events-none z-50">"admin.kgdev.me"</div>
            <header class="card-outer border-x-0 border-t-0 px-6 py-4 flex items-center justify-between z-10">
                <div class="flex items-center gap-8 w-full max-w-6xl mx-auto">
                    <div class="flex flex-col">
                        <div class="cyber-title cursor-default">
                            <span class="cyber-title-text" attr:data-text="RAG_ADMIN">
                                "RAG_ADMIN"
                            </span>
                        </div>
                    </div>
                    <nav class="flex gap-6 text-sm font-medium pt-3 flex-1">
                        <A href="/" attr:class="hover:text-[var(--color-accent)] transition-colors">
                            <span class="opacity-50 mr-1">"01"</span>"POSTS"
                        </A>
                        <A href="/embed" attr:class="hover:text-[var(--color-accent)] transition-colors">
                            <span class="opacity-50 mr-1">"02"</span>"EMBED_TEST"
                        </A>
                        <A href="/settings" attr:class="hover:text-[var(--color-accent)] transition-colors ml-auto">
                            <span class="opacity-50 mr-1">"03"</span>"SETTINGS"
                        </A>
                    </nav>
                </div>
            </header>
            <main class="flex-1 px-6 py-8 max-w-6xl mx-auto w-full relative">
                <div class="relative z-10">{children()}</div>
            </main>
        </div>
    }
}
