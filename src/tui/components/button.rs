use crate::tui::{Render, VirtualDomElement};

/// `Button` component that implements `Render` trait.
pub struct Button {
  pub width: u32,
  pub height: u32,
  pub label: String,
}

impl Render for Button {
  fn render(&self) -> VirtualDomElement {
    VirtualDomElement::new("button")
      .add_child(VirtualDomElement::new("width").value(&self.width.to_string()))
      .add_child(VirtualDomElement::new("height").value(&self.height.to_string()))
      .add_child(VirtualDomElement::new("label").value(&self.label.to_string()))
  }
}
