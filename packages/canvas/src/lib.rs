use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_hooks::*;
use dioxus_html as dioxus_elements;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::Duration;
use wasm_bindgen::JsCast;
use web_sys::window;
use web_sys::CanvasRenderingContext2d;
use web_sys::HtmlCanvasElement;

#[derive(Props)]
struct CanvasProps<'a> {
    children: Element<'a>,
}

fn Canvas<'a, C: CanvasHandler + 'static>(cx: Scope<'a, CanvasProps<'a>>) -> Element<'a> {
    let canvas = use_ref(&cx, || None);
    let id = cx.scope_id().0;
    let lzy = C::create(id);
    let canvas_clone = canvas.clone();
    use_future(&cx, (), |_| async move {
        // futures will not be polled until after the first render in the web renderer...
        tokio::time::sleep(Duration::from_millis(0)).await;
        canvas_clone.set(Some(C::onmount(id)));
        cx.provide_context(CanvasHandle::new());
    });
    // wait to render children until after the canvas is mounted

    if let Some(lzy) = lzy {
        cx.render(lzy)
    } else {
        None
    }
}

/// A handle to the canvas
pub struct CanvasHandle<C: CanvasHandler>(Rc<Mutex<Canvas<C>>>);

impl<C: CanvasHandler> CanvasHandle<C> {
    fn new(id: usize, handler: C) {
        let canvas = Canvas::new(id, handler);
        let canvas_rc = Rc::new(Mutex::new(canvas));
    }
}

pub struct Canvas<C: CanvasHandler> {
    id: usize,
    lzy: Option<C>,
    command_queue: Vec<CanvasCommand>,
}

impl<C: CanvasHandler> Canvas<C> {}

enum CanvasCommand {}

trait CanvasHandler {
    type RenderContext: piet::RenderContext;

    fn create<'a, 'b>(id: usize) -> Option<LazyNodes<'a, 'b>>;

    fn draw(&mut self, id: usize) -> &mut Self::RenderContext;

    fn onmount(id: usize) -> Self;

    // could add more methods here to handle filters, etc.
}

struct WebHandler {
    render_ctx: piet_web::WebRenderContext<'static>,
}

impl CanvasHandler for WebHandler {
    type RenderContext = piet_web::WebRenderContext<'static>;

    fn create<'b, 'c>(id: usize) -> Option<LazyNodes<'b, 'c>> {
        Some(rsx! {
            canvas{
                id: "dioxus-canvas-{id}"
            }
        })
    }

    fn onmount(id: usize) -> WebHandler {
        let window = window().unwrap();
        let canvas = window
            .document()
            .unwrap()
            .get_element_by_id(&format!("dioxus-canvas-{}", id))
            .unwrap();
        let canvas_html: HtmlCanvasElement = canvas.dyn_into().unwrap();
        let context: CanvasRenderingContext2d = canvas_html
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap();
        let context = piet_web::WebRenderContext::new(context, window);
        Self {
            render_ctx: context,
        }
    }

    fn draw(&mut self, id: usize) -> &mut Self::RenderContext {
        &mut self.render_ctx
    }
}
