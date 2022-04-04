/*
- [ ] pub display: Display,
- [x] pub position_type: PositionType,  --> kinda, stretch doesnt support everything
- [ ] pub direction: Direction,

- [x] pub flex_direction: FlexDirection,
- [x] pub flex_wrap: FlexWrap,
- [x] pub flex_grow: f32,
- [x] pub flex_shrink: f32,
- [x] pub flex_basis: Dimension,

- [x] pub overflow: Overflow, ---> kinda implemented... stretch doesnt have support for directional overflow

- [x] pub align_items: AlignItems,
- [x] pub align_self: AlignSelf,
- [x] pub align_content: AlignContent,

- [x] pub margin: Rect<Dimension>,
- [x] pub padding: Rect<Dimension>,

- [x] pub justify_content: JustifyContent,
- [ ] pub position: Rect<Dimension>,
- [x] pub border: Rect<Dimension>,

- [ ] pub size: Size<Dimension>, ----> ??? seems to only be relevant for input?
- [ ] pub min_size: Size<Dimension>,
- [ ] pub max_size: Size<Dimension>,

- [ ] pub aspect_ratio: Number,
*/

use dioxus_core::{Attribute, VNode};
use dioxus_native_core::{
    layout_attributes::{parse_value, UnitSystem},
    real_dom::PushedDownState,
};

use crate::style::{RinkColor, RinkStyle};

#[derive(Default, Clone, PartialEq, Debug)]
pub struct StyleModifier {
    pub style: RinkStyle,
    pub modifier: TuiModifier,
}

impl PushedDownState for StyleModifier {
    type Ctx = ();

    fn reduce(&mut self, parent: Option<&Self>, vnode: &VNode, _ctx: &mut Self::Ctx) {
        *self = StyleModifier::default();
        if parent.is_some() {
            self.style.fg = None;
        }
        if let VNode::Element(el) = vnode {
            // handle text modifier elements
            if el.namespace.is_none() {
                match el.tag {
                    "b" => apply_style_attributes("font-weight", "bold", self),
                    "strong" => apply_style_attributes("font-weight", "bold", self),
                    "u" => apply_style_attributes("text-decoration", "underline", self),
                    "ins" => apply_style_attributes("text-decoration", "underline", self),
                    "del" => apply_style_attributes("text-decoration", "line-through", self),
                    "i" => apply_style_attributes("font-style", "italic", self),
                    "em" => apply_style_attributes("font-style", "italic", self),
                    "mark" => {
                        apply_style_attributes("background-color", "rgba(241, 231, 64, 50%)", self)
                    }
                    _ => (),
                }
            }

            // gather up all the styles from the attribute list
            for &Attribute { name, value, .. } in el.attributes {
                apply_style_attributes(name, value, self);
            }
        }

        // keep the text styling from the parent element
        if let Some(parent) = parent {
            let mut new_style = self.style.merge(parent.style);
            new_style.bg = self.style.bg;
            self.style = new_style;
        }
    }
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct TuiModifier {
    pub borders: Borders,
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Borders {
    pub top: BorderEdge,
    pub right: BorderEdge,
    pub bottom: BorderEdge,
    pub left: BorderEdge,
}

impl Borders {
    fn slice(&mut self) -> [&mut BorderEdge; 4] {
        [
            &mut self.top,
            &mut self.right,
            &mut self.bottom,
            &mut self.left,
        ]
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct BorderEdge {
    pub color: Option<RinkColor>,
    pub style: BorderStyle,
    pub width: UnitSystem,
    pub radius: UnitSystem,
}

impl Default for BorderEdge {
    fn default() -> Self {
        Self {
            color: None,
            style: BorderStyle::None,
            width: UnitSystem::Point(0.0),
            radius: UnitSystem::Point(0.0),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BorderStyle {
    Dotted,
    Dashed,
    Solid,
    Double,
    Groove,
    Ridge,
    Inset,
    Outset,
    Hidden,
    None,
}

impl BorderStyle {
    pub fn symbol_set(&self) -> Option<tui::symbols::line::Set> {
        use tui::symbols::line::*;
        const DASHED: Set = Set {
            horizontal: "╌",
            vertical: "╎",
            ..NORMAL
        };
        const DOTTED: Set = Set {
            horizontal: "┈",
            vertical: "┊",
            ..NORMAL
        };
        match self {
            BorderStyle::Dotted => Some(DOTTED),
            BorderStyle::Dashed => Some(DASHED),
            BorderStyle::Solid => Some(NORMAL),
            BorderStyle::Double => Some(DOUBLE),
            BorderStyle::Groove => Some(NORMAL),
            BorderStyle::Ridge => Some(NORMAL),
            BorderStyle::Inset => Some(NORMAL),
            BorderStyle::Outset => Some(NORMAL),
            BorderStyle::Hidden => None,
            BorderStyle::None => None,
        }
    }
}

/// applies the entire html namespace defined in dioxus-html
pub fn apply_style_attributes(
    //
    name: &str,
    value: &str,
    style: &mut StyleModifier,
) {
    match name {
        "animation"
        | "animation-delay"
        | "animation-direction"
        | "animation-duration"
        | "animation-fill-mode"
        | "animation-iteration-count"
        | "animation-name"
        | "animation-play-state"
        | "animation-timing-function" => apply_animation(name, value, style),

        "backface-visibility" => {}

        "background"
        | "background-attachment"
        | "background-clip"
        | "background-color"
        | "background-image"
        | "background-origin"
        | "background-position"
        | "background-repeat"
        | "background-size" => apply_background(name, value, style),

        "border"
        | "border-bottom"
        | "border-bottom-color"
        | "border-bottom-left-radius"
        | "border-bottom-right-radius"
        | "border-bottom-style"
        | "border-bottom-width"
        | "border-collapse"
        | "border-color"
        | "border-image"
        | "border-image-outset"
        | "border-image-repeat"
        | "border-image-slice"
        | "border-image-source"
        | "border-image-width"
        | "border-left"
        | "border-left-color"
        | "border-left-style"
        | "border-left-width"
        | "border-radius"
        | "border-right"
        | "border-right-color"
        | "border-right-style"
        | "border-right-width"
        | "border-spacing"
        | "border-style"
        | "border-top"
        | "border-top-color"
        | "border-top-left-radius"
        | "border-top-right-radius"
        | "border-top-style"
        | "border-top-width"
        | "border-width" => apply_border(name, value, style),

        "bottom" => {}
        "box-shadow" => {}
        "box-sizing" => {}
        "caption-side" => {}
        "clear" => {}
        "clip" => {}

        "color" => {
            if let Ok(c) = value.parse() {
                style.style.fg.replace(c);
            }
        }

        "columns" => {}

        "content" => {}
        "counter-increment" => {}
        "counter-reset" => {}

        "cursor" => {}

        "empty-cells" => {}

        "float" => {}

        "font" | "font-family" | "font-size" | "font-size-adjust" | "font-stretch"
        | "font-style" | "font-variant" | "font-weight" => apply_font(name, value, style),

        "letter-spacing" => {}
        "line-height" => {}

        "list-style" | "list-style-image" | "list-style-position" | "list-style-type" => {}

        "opacity" => {}
        "order" => {}
        "outline" => {}

        "outline-color" | "outline-offset" | "outline-style" | "outline-width" => {}

        "page-break-after" | "page-break-before" | "page-break-inside" => {}

        "perspective" | "perspective-origin" => {}

        "pointer-events" => {}

        "quotes" => {}
        "resize" => {}
        "tab-size" => {}
        "table-layout" => {}

        "text-align"
        | "text-align-last"
        | "text-decoration"
        | "text-decoration-color"
        | "text-decoration-line"
        | "text-decoration-style"
        | "text-indent"
        | "text-justify"
        | "text-overflow"
        | "text-shadow"
        | "text-transform" => apply_text(name, value, style),

        "transition"
        | "transition-delay"
        | "transition-duration"
        | "transition-property"
        | "transition-timing-function" => apply_transition(name, value, style),

        "visibility" => {}
        "white-space" => {}
        _ => {}
    }
}

fn apply_background(name: &str, value: &str, style: &mut StyleModifier) {
    match name {
        "background-color" => {
            if let Ok(c) = value.parse() {
                style.style.bg.replace(c);
            }
        }
        "background" => {}
        "background-attachment" => {}
        "background-clip" => {}
        "background-image" => {}
        "background-origin" => {}
        "background-position" => {}
        "background-repeat" => {}
        "background-size" => {}
        _ => {}
    }
}

fn apply_border(name: &str, value: &str, style: &mut StyleModifier) {
    fn parse_border_style(v: &str) -> BorderStyle {
        match v {
            "dotted" => BorderStyle::Dotted,
            "dashed" => BorderStyle::Dashed,
            "solid" => BorderStyle::Solid,
            "double" => BorderStyle::Double,
            "groove" => BorderStyle::Groove,
            "ridge" => BorderStyle::Ridge,
            "inset" => BorderStyle::Inset,
            "outset" => BorderStyle::Outset,
            "none" => BorderStyle::None,
            "hidden" => BorderStyle::Hidden,
            _ => todo!(),
        }
    }
    match name {
        "border" => {}
        "border-bottom" => {}
        "border-bottom-color" => {
            if let Ok(c) = value.parse() {
                style.modifier.borders.bottom.color = Some(c);
            }
        }
        "border-bottom-left-radius" => {
            if let Some(v) = parse_value(value) {
                style.modifier.borders.left.radius = v;
            }
        }
        "border-bottom-right-radius" => {
            if let Some(v) = parse_value(value) {
                style.modifier.borders.right.radius = v;
            }
        }
        "border-bottom-style" => style.modifier.borders.bottom.style = parse_border_style(value),
        "border-bottom-width" => {
            if let Some(v) = parse_value(value) {
                style.modifier.borders.bottom.width = v;
            }
        }
        "border-collapse" => {}
        "border-color" => {
            let values: Vec<_> = value.split(' ').collect();
            if values.len() == 1 {
                if let Ok(c) = values[0].parse() {
                    style
                        .modifier
                        .borders
                        .slice()
                        .iter_mut()
                        .for_each(|b| b.color = Some(c));
                }
            } else {
                for (v, b) in values
                    .into_iter()
                    .zip(style.modifier.borders.slice().iter_mut())
                {
                    if let Ok(c) = v.parse() {
                        b.color = Some(c);
                    }
                }
            }
        }
        "border-image" => {}
        "border-image-outset" => {}
        "border-image-repeat" => {}
        "border-image-slice" => {}
        "border-image-source" => {}
        "border-image-width" => {}
        "border-left" => {}
        "border-left-color" => {
            if let Ok(c) = value.parse() {
                style.modifier.borders.left.color = Some(c);
            }
        }
        "border-left-style" => style.modifier.borders.left.style = parse_border_style(value),
        "border-left-width" => {
            if let Some(v) = parse_value(value) {
                style.modifier.borders.left.width = v;
            }
        }
        "border-radius" => {
            let values: Vec<_> = value.split(' ').collect();
            if values.len() == 1 {
                if let Some(r) = parse_value(values[0]) {
                    style
                        .modifier
                        .borders
                        .slice()
                        .iter_mut()
                        .for_each(|b| b.radius = r);
                }
            } else {
                for (v, b) in values
                    .into_iter()
                    .zip(style.modifier.borders.slice().iter_mut())
                {
                    if let Some(r) = parse_value(v) {
                        b.radius = r;
                    }
                }
            }
        }
        "border-right" => {}
        "border-right-color" => {
            if let Ok(c) = value.parse() {
                style.modifier.borders.right.color = Some(c);
            }
        }
        "border-right-style" => style.modifier.borders.right.style = parse_border_style(value),
        "border-right-width" => {
            if let Some(v) = parse_value(value) {
                style.modifier.borders.right.width = v;
            }
        }
        "border-spacing" => {}
        "border-style" => {
            let values: Vec<_> = value.split(' ').collect();
            if values.len() == 1 {
                let border_style = parse_border_style(values[0]);
                style
                    .modifier
                    .borders
                    .slice()
                    .iter_mut()
                    .for_each(|b| b.style = border_style);
            } else {
                for (v, b) in values
                    .into_iter()
                    .zip(style.modifier.borders.slice().iter_mut())
                {
                    b.style = parse_border_style(v);
                }
            }
        }
        "border-top" => {}
        "border-top-color" => {
            if let Ok(c) = value.parse() {
                style.modifier.borders.top.color = Some(c);
            }
        }
        "border-top-left-radius" => {
            if let Some(v) = parse_value(value) {
                style.modifier.borders.left.radius = v;
            }
        }
        "border-top-right-radius" => {
            if let Some(v) = parse_value(value) {
                style.modifier.borders.right.radius = v;
            }
        }
        "border-top-style" => style.modifier.borders.top.style = parse_border_style(value),
        "border-top-width" => {
            if let Some(v) = parse_value(value) {
                style.modifier.borders.top.width = v;
            }
        }
        "border-width" => {
            let values: Vec<_> = value.split(' ').collect();
            if values.len() == 1 {
                if let Some(w) = parse_value(values[0]) {
                    style
                        .modifier
                        .borders
                        .slice()
                        .iter_mut()
                        .for_each(|b| b.width = w);
                }
            } else {
                for (v, width) in values
                    .into_iter()
                    .zip(style.modifier.borders.slice().iter_mut())
                {
                    if let Some(w) = parse_value(v) {
                        width.width = w;
                    }
                }
            }
        }
        _ => (),
    }
}

fn apply_animation(name: &str, _value: &str, _style: &mut StyleModifier) {
    match name {
        "animation" => {}
        "animation-delay" => {}
        "animation-direction =>{}" => {}
        "animation-duration" => {}
        "animation-fill-mode" => {}
        "animation-itera =>{}tion-count" => {}
        "animation-name" => {}
        "animation-play-state" => {}
        "animation-timing-function" => {}
        _ => {}
    }
}

fn apply_font(name: &str, value: &str, style: &mut StyleModifier) {
    use tui::style::Modifier;
    match name {
        "font" => (),
        "font-family" => (),
        "font-size" => (),
        "font-size-adjust" => (),
        "font-stretch" => (),
        "font-style" => match value {
            "italic" => style.style = style.style.add_modifier(Modifier::ITALIC),
            "oblique" => style.style = style.style.add_modifier(Modifier::ITALIC),
            _ => (),
        },
        "font-variant" => todo!(),
        "font-weight" => match value {
            "bold" => style.style = style.style.add_modifier(Modifier::BOLD),
            "normal" => style.style = style.style.remove_modifier(Modifier::BOLD),
            _ => (),
        },
        _ => (),
    }
}

fn apply_text(name: &str, value: &str, style: &mut StyleModifier) {
    use tui::style::Modifier;

    match name {
        "text-align" => todo!(),
        "text-align-last" => todo!(),
        "text-decoration" | "text-decoration-line" => {
            for v in value.split(' ') {
                match v {
                    "line-through" => style.style = style.style.add_modifier(Modifier::CROSSED_OUT),
                    "underline" => style.style = style.style.add_modifier(Modifier::UNDERLINED),
                    _ => (),
                }
            }
        }
        "text-decoration-color" => todo!(),
        "text-decoration-style" => todo!(),
        "text-indent" => todo!(),
        "text-justify" => todo!(),
        "text-overflow" => todo!(),
        "text-shadow" => todo!(),
        "text-transform" => todo!(),
        _ => todo!(),
    }
}

fn apply_transition(_name: &str, _value: &str, _style: &mut StyleModifier) {
    todo!()
}