# r3bl_simple_logger

## Why R3BL?

<img src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/r3bl-term.svg?raw=true" height="256px">

<!-- R3BL TUI library & suite of apps focused on developer productivity -->

<span style="color:#FD2F53">R</span><span style="color:#FC2C57">3</span><span style="color:#FB295B">B</span><span style="color:#FA265F">L</span><span style="color:#F92363">
</span><span style="color:#F82067">T</span><span style="color:#F61D6B">U</span><span style="color:#F51A6F">I</span><span style="color:#F31874">
</span><span style="color:#F11678">l</span><span style="color:#EF137C">i</span><span style="color:#ED1180">b</span><span style="color:#EB0F84">r</span><span style="color:#E90D89">a</span><span style="color:#E60B8D">r</span><span style="color:#E40A91">y</span><span style="color:#E10895">
</span><span style="color:#DE0799">&amp;</span><span style="color:#DB069E">
</span><span style="color:#D804A2">s</span><span style="color:#D503A6">u</span><span style="color:#D203AA">i</span><span style="color:#CF02AE">t</span><span style="color:#CB01B2">e</span><span style="color:#C801B6">
</span><span style="color:#C501B9">o</span><span style="color:#C101BD">f</span><span style="color:#BD01C1">
</span><span style="color:#BA01C4">a</span><span style="color:#B601C8">p</span><span style="color:#B201CB">p</span><span style="color:#AE02CF">s</span><span style="color:#AA03D2">
</span><span style="color:#A603D5">f</span><span style="color:#A204D8">o</span><span style="color:#9E06DB">c</span><span style="color:#9A07DE">u</span><span style="color:#9608E1">s</span><span style="color:#910AE3">e</span><span style="color:#8D0BE6">d</span><span style="color:#890DE8">
</span><span style="color:#850FEB">o</span><span style="color:#8111ED">n</span><span style="color:#7C13EF">
</span><span style="color:#7815F1">d</span><span style="color:#7418F3">e</span><span style="color:#701AF5">v</span><span style="color:#6B1DF6">e</span><span style="color:#6720F8">l</span><span style="color:#6322F9">o</span><span style="color:#5F25FA">p</span><span style="color:#5B28FB">e</span><span style="color:#572CFC">r</span><span style="color:#532FFD">
</span><span style="color:#4F32FD">p</span><span style="color:#4B36FE">r</span><span style="color:#4739FE">o</span><span style="color:#443DFE">d</span><span style="color:#4040FE">u</span><span style="color:#3C44FE">c</span><span style="color:#3948FE">t</span><span style="color:#354CFE">i</span><span style="color:#324FFD">v</span><span style="color:#2E53FD">i</span><span style="color:#2B57FC">t</span><span style="color:#285BFB">y</span>

We are working on building command line apps in Rust which have rich text user interfaces (TUI).
We want to lean into the terminal as a place of productivity, and build all kinds of awesome
apps for it.

1. 🔮 Instead of just building one app, we are building a library to enable any kind of rich TUI
   development w/ a twist: taking concepts that work really well for the frontend mobile and web
   development world and re-imagining them for TUI & Rust.

   - Taking inspiration from things like [React](https://react.dev/),
     [SolidJS](https://www.solidjs.com/),
     [Elm](https://guide.elm-lang.org/architecture/),
     [iced-rs](https://docs.rs/iced/latest/iced/), [Jetpack
     Compose](https://developer.android.com/compose),
     [JSX](https://ui.dev/imperative-vs-declarative-programming),
     [CSS](https://www.w3.org/TR/CSS/#css), but making everything async (so they can
     be run in parallel & concurrent via [Tokio](https://crates.io/crates/tokio)).
   - Even the thread running the main event loop doesn't block since it is async.
   - Using proc macros to create DSLs to implement something inspired by
     [CSS](https://www.w3.org/TR/CSS/#css) &
     [JSX](https://ui.dev/imperative-vs-declarative-programming).

2. 🌎 We are building apps to enhance developer productivity & workflows.

   - The idea here is not to rebuild `tmux` in Rust (separate processes mux'd onto a
     single terminal window). Rather it is to build a set of integrated "apps" (or
     "tasks") that run in the same process that renders to one terminal window.
   - Inside of this terminal window, we can implement things like "app" switching,
     routing, tiling layout, stacking layout, etc. so that we can manage a lot of TUI
     apps (which are tightly integrated) that are running in the same process, in the
     same window. So you can imagine that all these "app"s have shared application
     state. Each "app" may also have its own local application state.
   - Here are some examples of the types of "app"s we plan to build (for which this
     infrastructure acts as the open source engine):
     1. Multi user text editors w/ syntax highlighting.
     2. Integrations w/ github issues.
     3. Integrations w/ calendar, email, contacts APIs.

All the crates in the `r3bl-open-core`
[repo](https://github.com/r3bl-org/r3bl-open-core/) provide lots of useful
functionality to help you build TUI (text user interface) apps, along w/ general
niceties & ergonomics that all Rustaceans 🦀 can enjoy 🎉.

## Table of contents

<!-- TOC -->

- [Introduction](#introduction)
- [Changelog](#changelog)
- [Learn how these crates are built, provide feedback](#learn-how-these-crates-are-built-provide-feedback)
- [How to customize or change logging implementation](#how-to-customize-or-change-logging-implementation)

<!-- /TOC -->

## Introduction
<a id="markdown-introduction" name="introduction"></a>

The simplest way to use this crate to log things and simply use the logging
facilities, is to use the
[`r3bl_rs_utils_core`](https://crates.io/crates/r3bl_rs_utils_core) crate, and not
this crate directly.
- Look at the
  [`r3bl_rs_utils_core::try_to_set_log_level`](https://docs.rs/r3bl_rs_utils_core/latest/r3bl_rs_utils_core/utils/file_logging/fn.try_to_set_log_level.html)
  function in the `r3bl_rs_utils_core` crate as the main entry point.
- By default, logging is disabled even if you call all the functions in the
  `file_logger` module in the `r3bl_rs_utils_core` crate: `log_debug`, `log_info`,
  `log_trace`, etc.

## Changelog
<a id="markdown-changelog" name="changelog"></a>

Please check out the
[changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl_simple_logger)
to see how the library has evolved over time.

## Learn how these crates are built, provide feedback
<a id="markdown-learn-how-these-crates-are-built-provide-feedback" name="learn-how-these-crates-are-built-provide-feedback"></a>

To learn how we built this crate, please take a look at the following resources.
- If you like consuming video content, here's our [YT channel](https://www.youtube.com/@developerlifecom). Please consider [subscribing](https://www.youtube.com/channel/CHANNEL_ID?sub_confirmation=1).
- If you like consuming written content, here's our developer [site](https://developerlife.com/). Please consider subscribing to our [newsletter](https://developerlife.com/subscribe.html).
- If you have questions, please join our [discord server](https://discord.gg/8M2ePAevaM).

## How to customize or change logging implementation
<a id="markdown-how-to-customize-or-change-logging-implementation" name="how-to-customize-or-change-logging-implementation"></a>

Under the hood the [`simplelog`](https://crates.io/crates/simplelog) crate is forked
and modified for use here.

The following are details for people who want to work on changing the underlying
behavior of the logging engine itself, and *not* for folks who just want to use this
crate.

`r3bl_simple_logger` provides a series of logging facilities, that can be easily
combined.

- `SimpleLogger` (very basic logger that logs to stdout)
- `TermLogger` (advanced terminal logger, that splits to stdout/err and has color
  support) (can be excluded on unsupported platforms)
- `WriteLogger` (logs to a given struct implementing `Write`, e.g. a file)
- `CombinedLogger` (can be used to form combinations of the above loggers)
- `TestLogger` (specialized logger for tests. Uses print!() / println!() for tests to
  be able to capture the output)

Only one Logger should be initialized of the start of your program through the
`Logger::init(...)` method. For the actual calling syntax take a look at the
documentation of the specific implementation(s) you want to use.

License: Apache-2.0
