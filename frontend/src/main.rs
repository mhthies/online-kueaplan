use leptos::*;
use uuid::Uuid;

fn main() {
    mount_to_body(|| view! { <App/> })
}

#[component]
fn App() -> impl IntoView {
    let example_kueas = vec![Kuea {
        id: Uuid::new_v4(),
        title: "Test".to_owned(),
        responsible_person: "Michael Thies".to_owned(),
    }];
    let (displayed_kueas, set_displayed_kueas) = create_signal(example_kueas);
    use leptos::html::*;

    (KueaTableView(displayed_kueas),)
}

// TODO replace with API datatype
#[derive(Clone)]
struct Kuea {
    pub id: Uuid,
    pub title: String,
    pub responsible_person: String,
}

// #[component]
fn KueaTableView(kueas: ReadSignal<Vec<Kuea>>) -> impl IntoView {
    use leptos::html::*;

    (table()
        .class("kuea-list", || true)
        .child(
            tr().child(th().child("When?"))
                .child(th().child("Where?"))
                .child(th().child("Title"))
                .child(th().child("Who?")),
        )
        .child(leptos_dom::Each::new(
            move || kueas.get(),
            |kuea| kuea.id,
            |kuea| {
                tr().child(td().child(""))
                    .child(td().child(""))
                    .child(td().child(kuea.title))
                    .child(td().child(kuea.responsible_person))
            },
        )),)
}
