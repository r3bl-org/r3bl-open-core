# Task: Enhanced Analytics for ch Binary

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Status](#status)
- [Background](#background)
- [Problem Statement](#problem-statement)
- [Proposed Solution](#proposed-solution)
  - [New Analytics Events](#new-analytics-events)
    - [Core Interaction Events](#core-interaction-events)
    - [Feature-Specific Events](#feature-specific-events)
- [Implementation Plan](#implementation-plan)
  - [Step 1: Update Analytics Action Enum](#step-1-update-analytics-action-enum)
  - [Step 2: Modify Main Command Handler](#step-2-modify-main-command-handler)
  - [Step 3: Update Selection Handler](#step-3-update-selection-handler)
  - [Step 4: Consider Metadata Enhancement (Optional)](#step-4-consider-metadata-enhancement-optional)
  - [Step 5: Testing Plan](#step-5-testing-plan)
- [Implementation Checklist](#implementation-checklist)
- [Success Metrics](#success-metrics)
- [Notes for Implementer](#notes-for-implementer)
- [Related Files](#related-files)
- [Questions to Resolve](#questions-to-resolve)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Status

- **Created**: 2025-08-16
- **Status**: Not Started
- **Priority**: Medium
- **Estimated Effort**: 2-3 hours

## Background

The `ch` binary currently has minimal analytics compared to `edi` and `giti`:

- **Current state**: Only tracks `ChAppStart` and `ChFailedToRun`
- **Gap**: No tracking of actual user interactions with the prompt selection feature
- **Goal**: Understand feature usage patterns to inform product decisions

## Problem Statement

We need better visibility into how users interact with the `ch` prompt selector to:

1. Measure feature adoption (% of users who select vs browse)
2. Understand success rates (selections vs cancellations)
3. Track image prompt usage to justify feature complexity
4. Identify UX improvement opportunities from usage patterns

## Proposed Solution

### New Analytics Events

#### Core Interaction Events

1. **ChPromptSelected**
   - Fired when: User successfully selects a prompt
   - Metadata:
     - `prompt_index`: Position in list (0-based)
     - `total_prompts`: Total number of prompts shown
     - `has_images`: Boolean indicating if prompt contains images

2. **ChSelectionCancelled**
   - Fired when: User cancels without selecting (ESC or Ctrl+C)
   - Metadata:
     - `total_prompts`: Number of prompts that were available
     - `time_spent_ms`: Optional - time between display and cancel

3. **ChNoPromptsFound**
   - Fired when: No prompts available for current project
   - Metadata:
     - `project_path`: Hashed project identifier

#### Feature-Specific Events

4. **ChImagePromptSelected**
   - Fired when: User selects a prompt containing images
   - Metadata:
     - `image_count`: Number of images in the prompt
     - `prompt_index`: Position in list
     - `total_prompts`: Total number of prompts

## Implementation Plan

### Step 1: Update Analytics Action Enum

**File**: `cmdr/src/analytics_client/analytics_action.rs`

```rust
pub enum AnalyticsAction {
    // ... existing variants ...
    ChAppStart,
    ChFailedToRun,
    // New variants to add:
    ChPromptSelected,
    ChSelectionCancelled,
    ChNoPromptsFound,
    ChImagePromptSelected,
}
```

Update the Display implementation to include string representations for new events.

### Step 2: Modify Main Command Handler

**File**: `cmdr/src/ch/choose_prompt.rs` **Function**: `handle_ch_command()`

Add analytics tracking at key decision points:

1. After checking for empty history:

```rust
if history.is_empty() {
    report_analytics::start_task_to_generate_event(
        String::new(),
        AnalyticsAction::ChNoPromptsFound,
    );
    return Ok(ChResult::NoPromptsFound { project_path });
}
```

2. When user cancels selection:

```rust
None => {
    report_analytics::start_task_to_generate_event(
        String::new(),
        AnalyticsAction::ChSelectionCancelled,
    );
    Ok(ChResult::SelectionCancelled { ... })
}
```

### Step 3: Update Selection Handler

**File**: `cmdr/src/ch/choose_prompt.rs` **Function**: `handle_selected_prompt()`

Track successful selections with context:

```rust
fn handle_selected_prompt(...) -> ChResult {
    // Existing logic to find and process selection

    // Check if prompt has images
    let has_images = image_handler::count_images_in_history_item(&history_item) > 0;

    // Fire appropriate analytics event
    if has_images {
        report_analytics::start_task_to_generate_event(
            format!("images:{},index:{},total:{}",
                    image_count, selected_index, total_prompts),
            AnalyticsAction::ChImagePromptSelected,
        );
    } else {
        report_analytics::start_task_to_generate_event(
            format!("index:{},total:{}", selected_index, total_prompts),
            AnalyticsAction::ChPromptSelected,
        );
    }

    // Continue with existing clipboard logic
}
```

### Step 4: Consider Metadata Enhancement (Optional)

If we need richer metadata, consider extending the analytics client to support structured metadata:

**File**: `cmdr/src/analytics_client/report_analytics.rs`

Could add a new function like:

```rust
pub fn start_task_to_generate_event_with_metadata(
    metadata: HashMap<String, String>,
    action: AnalyticsAction,
)
```

### Step 5: Testing Plan

1. **Unit Tests**: Mock analytics calls in tests
2. **Integration Tests**: Verify events fire in correct scenarios
3. **Manual Testing Checklist**:
   - [ ] Launch ch with prompts → verify ChAppStart
   - [ ] Select a regular prompt → verify ChPromptSelected
   - [ ] Select an image prompt → verify ChImagePromptSelected
   - [ ] Press ESC to cancel → verify ChSelectionCancelled
   - [ ] Run ch in empty project → verify ChNoPromptsFound
   - [ ] Force an error → verify ChFailedToRun

## Implementation Checklist

- [ ] Add new event variants to AnalyticsAction enum
- [ ] Implement ChPromptSelected event tracking
- [ ] Implement ChSelectionCancelled event tracking
- [ ] Implement ChNoPromptsFound event tracking
- [ ] Implement ChImagePromptSelected event tracking
- [ ] Add metadata collection for prompt list size
- [ ] Update analytics action Display implementation
- [ ] Test all analytics events locally
- [ ] Update documentation if needed

## Success Metrics

After implementation, we should be able to answer:

1. What % of ch launches result in prompt selection?
2. What's the cancellation rate?
3. How often are image prompts used?
4. What's the typical prompt list size?
5. Do users tend to select recent prompts (low index) or scroll deeper?

## Notes for Implementer

- Keep analytics lightweight - don't impact performance
- Consider privacy - don't send actual prompt content
- Use existing analytics infrastructure patterns from edi/giti
- Test with `--no-analytics` flag to ensure it properly disables
- Consider rate limiting if events could fire rapidly

## Related Files

- Current analytics implementation: `cmdr/src/bin/ch.rs:39-42, 74-77`
- Analytics client: `cmdr/src/analytics_client/`
- Main ch logic: `cmdr/src/ch/choose_prompt.rs`
- Event enum: `cmdr/src/analytics_client/analytics_action.rs`

## Questions to Resolve

1. Should we track time spent before selection/cancellation?
2. Should we differentiate between ESC and Ctrl+C cancellations?
3. Do we need to track scroll events or just final selection?
4. Should prompt position be 0-based or 1-based in analytics?
