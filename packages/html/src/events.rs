macro_rules! impl_event {
    (
        $data:ty;
        $(
            $( #[$attr:meta] )*
            $name:ident
        )*
    ) => {
        $(
            $( #[$attr] )*
            #[inline]
            pub fn $name<'a, P>(_cx: &'a ::dioxus_core::ScopeState, mut _f: impl crate::IntoHandler<'a, $data, P>) -> ::dioxus_core::Attribute<'a> {
                ::dioxus_core::Attribute {
                    name: stringify!($name),
                    value: _f.into_handler(_cx),
                    namespace: None,
                    mounted_element: Default::default(),
                    volatile: false,
                }
            }
        )*
    };
}

mod animation;
mod clipboard;
mod composition;
mod drag;
mod focus;
mod form;
mod image;
mod keyboard;
mod media;
mod mouse;
mod pointer;
mod scroll;
mod selection;
mod toggle;
mod touch;
mod transition;
mod wheel;

pub use animation::*;
pub use clipboard::*;
pub use composition::*;
use dioxus_core::AttributeValue;
pub use drag::*;
pub use focus::*;
pub use form::*;
pub use image::*;
pub use keyboard::*;
pub use media::*;
pub use mouse::*;
pub use pointer::*;
pub use scroll::*;
pub use selection::*;
pub use toggle::*;
pub use touch::*;
pub use transition::*;
pub use wheel::*;

pub fn event_bubbles(evt: &str) -> bool {
    match evt {
        "copy" => true,
        "cut" => true,
        "paste" => true,
        "compositionend" => true,
        "compositionstart" => true,
        "compositionupdate" => true,
        "keydown" => true,
        "keypress" => true,
        "keyup" => true,
        "focus" => false,
        "focusout" => true,
        "focusin" => true,
        "blur" => false,
        "change" => true,
        "input" => true,
        "invalid" => true,
        "reset" => true,
        "submit" => true,
        "click" => true,
        "contextmenu" => true,
        "doubleclick" => true,
        "dblclick" => true,
        "drag" => true,
        "dragend" => true,
        "dragenter" => false,
        "dragexit" => false,
        "dragleave" => true,
        "dragover" => true,
        "dragstart" => true,
        "drop" => true,
        "mousedown" => true,
        "mouseenter" => false,
        "mouseleave" => false,
        "mousemove" => true,
        "mouseout" => true,
        "scroll" => false,
        "mouseover" => true,
        "mouseup" => true,
        "pointerdown" => true,
        "pointermove" => true,
        "pointerup" => true,
        "pointercancel" => true,
        "gotpointercapture" => true,
        "lostpointercapture" => true,
        "pointerenter" => false,
        "pointerleave" => false,
        "pointerover" => true,
        "pointerout" => true,
        "select" => true,
        "touchcancel" => true,
        "touchend" => true,
        "touchmove" => true,
        "touchstart" => true,
        "wheel" => true,
        "abort" => false,
        "canplay" => false,
        "canplaythrough" => false,
        "durationchange" => false,
        "emptied" => false,
        "encrypted" => true,
        "ended" => false,
        "error" => false,
        "loadeddata" => false,
        "loadedmetadata" => false,
        "loadstart" => false,
        "pause" => false,
        "play" => false,
        "playing" => false,
        "progress" => false,
        "ratechange" => false,
        "seeked" => false,
        "seeking" => false,
        "stalled" => false,
        "suspend" => false,
        "timeupdate" => false,
        "volumechange" => false,
        "waiting" => false,
        "animationstart" => true,
        "animationend" => true,
        "animationiteration" => true,
        "transitionend" => true,
        "toggle" => true,
        _ => true,
    }
}

use std::future::Future;

#[doc(hidden)]
pub trait IntoHandler<'a, E, P>: Sized {
    fn into_handler(self, cx: &'a ::dioxus_core::ScopeState) -> AttributeValue<'a>;
}

impl<'a, E> IntoHandler<'a, E, ()>
    for &'a dioxus_core::prelude::EventHandler<'a, dioxus_core::Event<E>>
{
    #[inline]
    fn into_handler(self, cx: &'a ::dioxus_core::ScopeState) -> AttributeValue<'a> {
        (move |evt| self.call(evt)).into_handler(cx)
    }
}

impl<'a, D: 'static, T, E: crate::EventReturn<T>, F: FnMut(::dioxus_core::Event<D>) -> E + 'a>
    IntoHandler<'a, D, (T, E)> for F
{
    #[inline]
    fn into_handler(mut self, cx: &'a ::dioxus_core::ScopeState) -> AttributeValue<'a> {
        cx.listener(move |data| (self)(data).spawn(cx))
    }
}

impl<'a, E, P, H> IntoHandler<'a, E, P> for Option<H>
where
    H: IntoHandler<'a, E, P> + Copy + 'a,
{
    #[inline]
    fn into_handler(self, cx: &'a ::dioxus_core::ScopeState) -> AttributeValue<'a> {
        match self {
            None => AttributeValue::None,
            Some(p) => p.into_handler(cx),
        }
    }
}

#[doc(hidden)]
pub trait EventReturn<P>: Sized {
    fn spawn(self, _cx: &dioxus_core::ScopeState) {}
}

impl EventReturn<()> for () {}
#[doc(hidden)]
pub struct AsyncMarker;

impl<T> EventReturn<AsyncMarker> for T
where
    T: Future<Output = ()> + 'static,
{
    #[inline]
    fn spawn(self, cx: &dioxus_core::ScopeState) {
        cx.spawn(self);
    }
}

#[test]
fn handler_type_works() {
    fn get_scope_state() -> dioxus_core::ScopeState {
        todo!()
    }

    fn takes_impl<'a, P>(_: impl IntoHandler<'a, String, P>) {
        todo!()
    }

    let cx = get_scope_state();

    let closure = |evt: dioxus_core::Event<String>| {
        println!("{}", evt.inner());
    };

    takes_impl(closure);

    let closure = |evt: dioxus_core::Event<String>| async move {
        println!("{}", evt.inner());
    };

    takes_impl(closure);
}
