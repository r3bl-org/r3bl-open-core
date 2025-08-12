// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Helper trait and impl to convert [`std::env::Args`] to a [`Vec<String>`] after
/// removing the first item (which is the path to the executable).
pub trait ArgsToStrings {
    fn filter_and_convert_to_strings(&self) -> Vec<String>;
    fn as_str(my_vec: &[String]) -> Vec<&str>;
}

impl ArgsToStrings for std::env::Args {
    fn filter_and_convert_to_strings(&self) -> Vec<String> {
        let mut list = std::env::args().collect::<Vec<String>>();
        if !list.is_empty() {
            list.remove(0);
        }
        list
    }

    fn as_str(my_vec: &[String]) -> Vec<&str> {
        my_vec.iter().map(String::as_str).collect::<Vec<&str>>()
    }
}
