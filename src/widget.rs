use crate::{Attribute, Node};
use sauron_vdom::builder::element;
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq)]
pub enum Widget {
    Vbox,
    Hbox,
    Button,
    Text(String),
    Block(String),
    TextInput(String),
    Checkbox(bool),
    Image(Vec<u8>),
}

pub fn widget<MSG>(
    widget: Widget,
    attrs: Vec<Attribute<MSG>>,
    children: Vec<Node<MSG>>,
) -> Node<MSG> {
    element(widget, attrs, children)
}

pub fn column<MSG>(attrs: Vec<Attribute<MSG>>, children: Vec<Node<MSG>>) -> Node<MSG> {
    widget(Widget::Vbox, attrs, children)
}

pub fn row<MSG>(attrs: Vec<Attribute<MSG>>, children: Vec<Node<MSG>>) -> Node<MSG> {
    widget(Widget::Hbox, attrs, children)
}

pub fn button<MSG>(attrs: Vec<Attribute<MSG>>) -> Node<MSG> {
    widget(Widget::Button, attrs, vec![])
}

pub fn text<MSG>(txt: &str) -> Node<MSG> {
    widget(Widget::Text(txt.to_string()), vec![], vec![])
}

pub fn text_input<MSG>(attrs: Vec<Attribute<MSG>>, txt: &str) -> Node<MSG> {
    widget(Widget::TextInput(txt.to_string()), attrs, vec![])
}

pub fn block<MSG>(title: &str) -> Node<MSG> {
    widget(Widget::Block(title.to_string()), vec![], vec![])
}

pub fn checkbox<MSG>(checked: bool) -> Node<MSG> {
    widget(Widget::Checkbox(checked), vec![], vec![])
}

pub fn image<MSG>(image: Vec<u8>) -> Node<MSG> {
    widget(Widget::Image(image), vec![], vec![])
}
