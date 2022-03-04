use crate::tui::{Render, VirtualDomElement};

/// `Text` component that implements `Render` trait.
pub struct Text {
  pub text: String,
}

impl Render for Text {
  fn render(&self) -> VirtualDomElement {
    VirtualDomElement::new("text").value(&self.text)
  }
}
