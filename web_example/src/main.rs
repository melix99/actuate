use actuate::{
    clone, use_state,
    web::{div, text},
    View,
};

#[derive(Clone)]
struct App;

impl View for App {
    fn body(&self, cx: &actuate::Scope) -> impl View {
        let (count, set_count) = use_state(cx, || 0);

        div(text(format!("High five count: {}", count))).on_click({
            clone!(count);
            move || set_count.set(count + 1)
        })
    }
}

fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();

    actuate::mount(
        App,
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .into(),
    )
}
