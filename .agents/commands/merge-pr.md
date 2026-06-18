# Merge PR

Invoke the `merge-pr` skill to push the current branch, create a GitHub Pull Request, and merge it into `main` via rebase.

This command streamlines the final step of a task by automatically handling the push (`git push -f`) and linear merging of the existing PR while simultaneously cleaning up the remote branch (`gh pr merge --rebase --delete-branch`).
