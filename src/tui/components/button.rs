use crate::tui::{Render, VDOMElement};

/// `Button` component that implements `Render` trait.
pub struct Button {
  pub width: u32,
  pub height: u32,
  pub label: String,
}

impl Render for Button {
  fn render(&self) -> VDOMElement {
    VDOMElement::new("button")
      .add_child(VDOMElement::new("width").value(&self.width.to_string()))
      .add_child(VDOMElement::new("height").value(&self.height.to_string()))
      .add_child(VDOMElement::new("label").value(&self.label.to_string()))
  }
}
