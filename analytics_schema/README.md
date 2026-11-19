# r3bl_analytics_schema

## Why R3BL?

<img src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/r3bl-term.svg?raw=true" height="256px">

<!-- R3BL TUI library & suite of apps focused on developer productivity -->

<span style="color: #FD2F53; background: linear-gradient(90deg, #FD2F53, #285BFB); -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent;">R3BL TUI library &amp; suite of apps focused on developer productivity</span>

Please read the main [README.md] of the `r3bl-open-core` monorepo and workspace to get
a better understanding of the context in which this crate is meant to exist.

## Table of contents

<!-- TOC -->

- [Introduction](#introduction)
- [Changelog](#changelog)
- [Learn how these crates are built, provide
  feedback](#learn-how-these-crates-are-built-provide-feedback)

<!-- /TOC -->

## Introduction

This crate is a shared dependency of a few crates in the R3BL ecosystem. It describes
data structures that are used to represent analytics data. These data structures are
created with anonymity in mind, and purposefully avoid including any personally
identifiable information (PII). Instead of adding privacy after the fact, these data
structures don't really allow for PII to be included in the first place. The intention
and philosophy behind these data structures is to be privacy-first, and provide a way
for R3BL to understand which products need more care and attention first, to be able
to deliver the best user experience to end users. And all the infrastructure supports
opt-out of this anonymized telemetry data collection.

Here are some other crates for which this crate is a dependency. To get a full list
run `rg "r3bl_analytics_schema" -g "Cargo.toml"` to get a list.
1. [`r3bl-cmdr`] crate.
2. The analytics backend for R3BL which is closed source.

## Changelog

Please check out the [changelog] to see how the library has evolved over time.

## Learn how these crates are built, provide feedback

To learn how we built this crate, please take a look at the following resources.
- If you like consuming video content, here's our [YT channel]. Please consider
  [subscribing].
- If you like consuming written content, here's our developer [site].

[`r3bl-cmdr`]: https://crates.io/crates/r3bl_cmdr
[changelog]: https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl_analytics_schema
[README.md]: https://github.com/r3bl-org/r3bl-open-core/blob/main/README.md
[site]: https://developerlife.com/
[subscribing]: https://www.youtube.com/channel/UCMcsxfCwzwDevc3NRqFgfEg?sub_confirmation=1
[YT channel]: https://www.youtube.com/@developerlifecom
## Analytics Schema

This crate defines the data structures used for analytics collection within
the R3BL ecosystem. It provides type-safe, serializable structures for
capturing user interactions and application events.

### Core Types

- [`AnalyticsRecord`] - Container for multiple analytics events
- [`AnalyticsEvent`] - Complete event with timestamp and UUID
- [`AnalyticsEventNoTimestamp`] - Event builder without auto-generated fields

### Features

- **Privacy-focused**: User IDs are not collected (empty `proxy_user_id`)
- **Memory efficient**: Uses `SmallVec` for small collections
- **Serializable**: All types implement Serde traits
- **Type-safe**: Strong typing prevents data corruption

### Usage

```rust
use r3bl_analytics_schema::*;

let mut record = AnalyticsRecord::default();
let event = AnalyticsEventNoTimestamp {
    proxy_user_id: String::new(),
    proxy_machine_id: "happy_panda_12".to_string(),
    action: "file_save".to_string(),
};
record.events.push(event.into());
```

License: Apache-2.0
