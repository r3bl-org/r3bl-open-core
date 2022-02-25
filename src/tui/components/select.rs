use crate::tui::{Render, VDOMElement};

/// `Select` component that implements `Render` trait.
pub struct Select {
  pub name: String,
  pub options: Vec<String>,
}

impl Render for Select {
  fn render(&self) -> VDOMElement {
    let options_to_vdom = self
      .options
      .iter()
      .map(|option| VDOMElement::new("option").value(option))
      .collect::<Vec<VDOMElement>>();
    VDOMElement::new("select").set_children(options_to_vdom)
  }
}
