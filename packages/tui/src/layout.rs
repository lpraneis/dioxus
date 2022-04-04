use dioxus_core::*;
use dioxus_native_core::layout_attributes::apply_layout_attributes;
use dioxus_native_core::real_dom::BubbledUpState;
use stretch2::prelude::*;

/// the size
#[derive(Clone, PartialEq, Default, Debug)]
pub struct StretchLayout {
    pub style: Style,
    pub node: Option<Node>,
}

impl BubbledUpState for StretchLayout {
    type Ctx = Stretch;

    /// Setup the layout
    fn reduce<'a, I>(&mut self, children: I, vnode: &VNode, stretch: &mut Self::Ctx)
    where
        I: Iterator<Item = &'a Self>,
        Self: 'a,
    {
        match vnode {
            VNode::Text(t) => {
                let char_len = t.text.chars().count();

                let style = Style {
                    size: Size {
                        // characters are 1 point tall
                        height: Dimension::Points(1.0),

                        // text is as long as it is declared
                        width: Dimension::Points(char_len as f32),
                    },
                    ..Default::default()
                };

                if let Some(n) = self.node {
                    if self.style != style {
                        stretch.set_style(n, style).unwrap();
                    }
                } else {
                    self.node = Some(stretch.new_node(style, &[]).unwrap());
                }

                self.style = style;
            }
            VNode::Element(el) => {
                // gather up all the styles from the attribute list
                let mut style = Style::default();

                for &Attribute { name, value, .. } in el.attributes {
                    apply_layout_attributes(name, value, &mut style);
                }

                // the root node fills the entire area
                if el.id.get() == Some(ElementId(0)) {
                    apply_layout_attributes("width", "100%", &mut style);
                    apply_layout_attributes("height", "100%", &mut style);
                }

                // Set all direct nodes as our children
                let mut child_layout = vec![];
                for l in children {
                    child_layout.push(l.node.unwrap());
                }

                if let Some(n) = self.node {
                    if stretch.children(n).unwrap() != child_layout {
                        stretch.set_children(n, &child_layout).unwrap();
                    }
                    if self.style != style {
                        stretch.set_style(n, style).unwrap();
                    }
                } else {
                    self.node = Some(stretch.new_node(style, &child_layout).unwrap());
                }

                self.style = style;
            }
            _ => (),
        }
    }
}