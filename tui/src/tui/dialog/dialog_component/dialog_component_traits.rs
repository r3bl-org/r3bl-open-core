// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use tokio::sync::mpsc::Sender;

use crate::{DialogBuffer, FlexBoxId, InlineString, TerminalWindowMainThreadSignal};

/// This marker trait is meant to be implemented by whatever state struct is being used to
/// store the dialog buffer for this re-usable editor component.
///
/// It is used in the `where` clause of the [`crate::DialogComponent`] to ensure that the
/// generic type `S` implements this trait, guaranteeing that it holds a single
/// [`DialogBuffer`].
pub trait HasDialogBuffers {
    fn get_mut_dialog_buffer(&mut self, id: FlexBoxId) -> Option<&mut DialogBuffer>;
}

#[derive(Debug)]
pub enum DialogChoice {
    Yes(InlineString),
    No,
}

pub type OnDialogPressFn<S, AS> = fn(
    DialogChoice,
    &mut S,
    main_thread_channel_sender: &mut Sender<TerminalWindowMainThreadSignal<AS>>,
);

pub type OnDialogEditorChangeFn<S, AS> = fn(
    &mut S,
    main_thread_channel_sender: &mut Sender<TerminalWindowMainThreadSignal<AS>>,
);
