# Review PR

Invoke the `review-pr` skill to systematically integrate and review a community Pull Request.

This command fetches the PR, breaks it down into a review plan, and ensures each fix is audited and applied to a clean local branch before eventually triggering `/merge-pr` to merge it.
