# r3bl_analytics_schema

## Why R3BL?

<img src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/r3bl-term.svg?raw=true" height="256px">

<!-- R3BL TUI library & suite of apps focused on developer productivity -->

<span style="color:#FD2F53">R</span><span style="color:#FC2C57">3</span><span
style="color:#FB295B">B</span><span style="color:#FA265F">L</span><span
style="color:#F92363"> </span><span style="color:#F82067">T</span><span
style="color:#F61D6B">U</span><span style="color:#F51A6F">I</span><span
style="color:#F31874"> </span><span style="color:#F11678">l</span><span
style="color:#EF137C">i</span><span style="color:#ED1180">b</span><span
style="color:#EB0F84">r</span><span style="color:#E90D89">a</span><span
style="color:#E60B8D">r</span><span style="color:#E40A91">y</span><span
style="color:#E10895"> </span><span style="color:#DE0799">&amp;</span><span
style="color:#DB069E"> </span><span style="color:#D804A2">s</span><span
style="color:#D503A6">u</span><span style="color:#D203AA">i</span><span
style="color:#CF02AE">t</span><span style="color:#CB01B2">e</span><span
style="color:#C801B6"> </span><span style="color:#C501B9">o</span><span
style="color:#C101BD">f</span><span style="color:#BD01C1"> </span><span
style="color:#BA01C4">a</span><span style="color:#B601C8">p</span><span
style="color:#B201CB">p</span><span style="color:#AE02CF">s</span><span
style="color:#AA03D2"> </span><span style="color:#A603D5">f</span><span
style="color:#A204D8">o</span><span style="color:#9E06DB">c</span><span
style="color:#9A07DE">u</span><span style="color:#9608E1">s</span><span
style="color:#910AE3">e</span><span style="color:#8D0BE6">d</span><span
style="color:#890DE8"> </span><span style="color:#850FEB">o</span><span
style="color:#8111ED">n</span><span style="color:#7C13EF"> </span><span
style="color:#7815F1">d</span><span style="color:#7418F3">e</span><span
style="color:#701AF5">v</span><span style="color:#6B1DF6">e</span><span
style="color:#6720F8">l</span><span style="color:#6322F9">o</span><span
style="color:#5F25FA">p</span><span style="color:#5B28FB">e</span><span
style="color:#572CFC">r</span><span style="color:#532FFD"> </span><span
style="color:#4F32FD">p</span><span style="color:#4B36FE">r</span><span
style="color:#4739FE">o</span><span style="color:#443DFE">d</span><span
style="color:#4040FE">u</span><span style="color:#3C44FE">c</span><span
style="color:#3948FE">t</span><span style="color:#354CFE">i</span><span
style="color:#324FFD">v</span><span style="color:#2E53FD">i</span><span
style="color:#2B57FC">t</span><span style="color:#285BFB">y</span>

Please read the
main [README.md](https://github.com/r3bl-org/r3bl-open-core/blob/main/README.md) of
the `r3bl-open-core` monorepo and workspace to get a better understanding of the
context in which this crate is meant to exist.

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

Here are some other crates for which this crate is a dependency (run `rg
"r3bl_analytics_schema" -g "Cargo.toml"` to get a list):
1. [`r3bl-cmdr`](https://crates.io/crates/r3bl_cmdr) crate.
1. The analytics backend for R3BL which is closed source.

## Changelog

Please check out the
[changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl_analytics_schema)
to see how the library has evolved over time.

## Learn how these crates are built, provide feedback

To learn how we built this crate, please take a look at the following resources.
- If you like consuming video content, here's our [YT channel](https://www.youtube.com/@developerlifecom).
  Please consider [subscribing](https://www.youtube.com/channel/CHANNEL_ID?sub_confirmation=1).
- If you like consuming written content, here's our developer [site](https://developerlife.com/).

License: Apache-2.0
