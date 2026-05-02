use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    view! {
        <div class="min-h-screen flex flex-col relative">
            <div class="fixed bottom-2 right-2 tech-label pointer-events-none z-50">"admin.kgdev.me"</div>
            <header class="card-outer border-x-0 border-t-0 px-6 py-4 flex items-center justify-between z-10">
                <div class="flex items-center gap-8">
                    <div class="flex flex-col">
                        <span class="text-xl font-bold tracking-tight" style="color: var(--color-accent-strong);">
                            "RAG_ADMIN"
                        </span>
                    </div>
                    <nav class="flex gap-6 text-sm font-medium pt-3">
                        <A href="/" attr:class="hover:text-[var(--color-accent)] transition-colors">
                            <span class="opacity-50 mr-1">"01"</span>"POSTS"
                        </A>
                        <A href="/settings" attr:class="hover:text-[var(--color-accent)] transition-colors">
                            <span class="opacity-50 mr-1">"02"</span>"SETTINGS"
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
