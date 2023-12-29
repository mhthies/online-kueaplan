use leptos::*;

fn main() {
    mount_to_body(|| view! { <App/> })
}

#[component]
fn App() -> impl IntoView {
    let (count, set_count) = create_signal(0);
    let double_count = move || count.get() * 2;
    use leptos::html::*;

    (
        ""
    )
}
