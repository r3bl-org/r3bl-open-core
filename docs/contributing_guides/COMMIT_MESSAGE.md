# Guide to writing a 📝 commit message

<a id="markdown-guide-to-writing-a-%F0%9F%93%9D-commit-message" name="guide-to-writing-a-%F0%9F%93%9D-commit-message"></a>

<!-- TOC -->

- [🎈 Why is it important to write good commit messages?](#-why-is-it-important-to-write-good-commit-messages)
- [✏️ How to write a commit message](#-how-to-write-a-commit-message)
  - [Commit message structure](#commit-message-structure)
  - [Subject](#subject)
  - [Body optional](#body-optional)
  - [Footer optional](#footer-optional)
  - [🌺 Examples](#-examples)

<!-- /TOC -->

## 🎈 Why is it important to write good commit messages?

<a id="markdown-%F0%9F%8E%88-why-is-it-important-to-write-good-commit-messages%3F" name="%F0%9F%8E%88-why-is-it-important-to-write-good-commit-messages%3F"></a>

Good commit messages are important because they help other developers understand the changes you've
made to the codebase. They also help you keep track of what you've done.

## ✏️ How to write a commit message

<a id="markdown-%E2%9C%8F%EF%B8%8F-how-to-write-a-commit-message" name="%E2%9C%8F%EF%B8%8F-how-to-write-a-commit-message"></a>

### Commit message structure

<a id="markdown-commit-message-structure" name="commit-message-structure"></a>

```
<subject>

<body> (optional)

<footer> (optional)
```

### Subject

<a id="markdown-subject" name="subject"></a>

- The subject should be a short description of the change.
  - The subject should be **less than 50 characters**.
- The subject should not end with a period.
- Use **imperative mood**. Using the imperative mood means to phrase your commit description as an
  order or instruction. For example, instead of writing "Fixed bug in user login", you would write
  **"Fix a bug in user login"**. This makes it clear and concise, describing what applying the
  commit will do rather than what has been done.
- Optional: Use reference to issue or pull request if relevant. For example, if you are fixing a bug
  that was reported in issue #123, you would write **"Fix bug in user login (fixes #123)"**.

### Body (optional)

<a id="markdown-body-optional" name="body-optional"></a>

- Separate the subject (the first line) from the body of the commit message with a blank line. This
  makes the commit message easier to read.
- Keep the commit message short and to the point. Aim for less than **72 characters per line**.

### Footer (optional)

<a id="markdown-footer-optional" name="footer-optional"></a>

- **Co-authors**: If the commit was also written by other contributors, you can give credit with
  co-author trailers. For example: Co-authored-by: name <name@example.com>.

### 🌺 Examples

<a id="markdown-%F0%9F%8C%BA-examples" name="%F0%9F%8C%BA-examples"></a>

Here are some examples of good commit messages:

```
Update login feature

This commit improves the user login by adding a 'Remember me' checkbox.

Co-authored-by: John Doe <johndoe@example.com>
```

```
Implement feature #123

This commit implements feature #123, which allows users to upload
images to their profile. It includes changes to the database schema,
API endpoints, and front-end code.
```

```
Add a section to STYLE_GUIDE.md to favor using enums

- Add a section to STYLE_GUIDE.md to favor using enums over booleans
- Add examples to the section and links to relevant resources
```

```
Add a new feature to the dashboard

This commit adds a new feature to the dashboard that allows users to
filter by date range. It also includes tests for the new feature.
```

```
Fix bug in login form

The login form was not properly validating user input, which could
lead to security vulnerabilities. This commit fixes the bug and adds
additional validation checks.
```

```
Refactor code for readability

This commit refactors the code to make it more readable and easier
to maintain. It includes changes to variable names, function names,
and code structure.
```

We hope this guide helps you write better commit messages 🎉!
