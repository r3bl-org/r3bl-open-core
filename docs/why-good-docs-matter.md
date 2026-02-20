# Why We Invest Heavily in Documentation

We devote significant effort, time, and resources to ensuring that our documentation is excellent.
This is not accidental - research consistently shows that documentation quality is the single most
important factor developers consider when evaluating open source projects.

## The evidence

### Documentation is the #1 evaluation criterion in the Rust ecosystem

[RFC 1824](https://rust-lang.github.io/rfcs/1824-crates.io-default-ranking.html), which defines how
crates are ranked on crates.io, surveyed the Rust community about what they look for when evaluating
crates. Documentation was **"by far, the most common attribute people said they considered"** - with
**104 mentions**, significantly more than any other factor. The RFC even proposed a "Well
Documented" badge for the top 20% of crates.

### 91% of practitioners depend on docs for adoption decisions

A [2024 academic study](https://arxiv.org/html/2403.03819v1) ("Does Documentation Matter? An
Empirical Study of Practitioners' Perspective on Open-Source Software Adoption") specifically
examined the link between documentation and OSS adoption:

- **All 10 interview participants** confirmed they rely on documentation when making adoption
  decisions.
- **91.18% of survey respondents** said they depend on documentation for integration decisions.
- On a 1-5 importance scale, interview participants gave documentation a **median score of 5**.

The study identified 9 adoption criteria practitioners seek in documentation: project maintenance,
compatibility, functionality, license compatibility, community adoption, usage examples, ease of
use, versioning, and performance.

### 93% of developers see poor docs as the top problem in open source

The [GitHub Open Source Survey](https://opensourcesurvey.org/2017/) (n=5,500+) found that
**incomplete or outdated documentation is a pervasive problem, observed by 93% of respondents**. Yet
60% of contributors rarely or never contribute to documentation. This gap is precisely what we aim
to fill by treating docs as a first-class deliverable.

### Documentation is how developers learn

The [Stack Overflow 2024 Developer Survey](https://survey.stackoverflow.co/2024/) found that **84%
of developers use technical documentation to learn**, and of those, **90% use the documentation
found in API and SDK packages**. Your docs.rs page is likely the first thing a developer reads about
your crate.

### Good docs build trust

Research on
[library adoption in public software repositories](https://journalofbigdata.springeropen.com/articles/10.1186/s40537-019-0201-8)
found that **"trust of a library plays an important role"** in adoption, and trust involves "the
assumption of a module's functional and non-functional correctness." Documentation is one of the
primary signals developers use to establish that trust - it demonstrates that the maintainers care
about the user experience, not just the code.

### Documentation and inclusivity

The GitHub survey also found that documentation that clearly explains project processes is **valued
more by underrepresented groups**, particularly women. Nearly 25% of the open source community has
limited English proficiency, making clear, straightforward documentation an inclusivity requirement,
not just a nice-to-have.

This is why we codified inclusivity as a core documentation principle. Our
[write-documentation](https://github.com/r3bl-org/r3bl-open-core/blob/main/.claude/skills/write-documentation/SKILL.md)
skill includes a "Pedagogical Links for Inclusivity" rule: link domain-specific terminology to
external references (typically Wikipedia) even when the concept seems "obvious." The cost of an
extra link is near zero; the cost of excluding a reader is high.

## What this means for r3bl-org

We treat documentation as a first-class artifact, not an afterthought:

- **Every public API** has rustdoc comments with usage examples.
- **Module-level docs** explain the "why" and architectural context, not just the "what".
- **Intra-doc links** connect related concepts so developers can navigate the API naturally.
- **Automated formatting** (`cargo rustdoc-fmt`) ensures consistent doc style across the codebase.
- **Doc tests** verify that every code example in documentation actually compiles and runs.
- **CI enforcement** - documentation builds are part of our continuous integration pipeline.
- **Inclusivity by default** - we
  [link pedagogical terms](https://github.com/r3bl-org/r3bl-open-core/blob/main/.claude/skills/write-documentation/SKILL.md#pedagogical-links-for-inclusivity)
  to external references so no reader is excluded by assumed knowledge.

We follow an inverted pyramid structure: high-level concepts at module and trait level, detailed
syntax examples at method level. This means developers can understand the big picture quickly and
dive deeper only when they need to.

Our complete documentation conventions are codified in the
[write-documentation skill](https://github.com/r3bl-org/r3bl-open-core/blob/main/.claude/skills/write-documentation/SKILL.md),
which covers voice and tone, prose style, structure, intra-doc links, constant conventions, and
formatting.

## Summary of evidence

| Evidence                                                                                                           | Key finding                                      | Strength               |
| :----------------------------------------------------------------------------------------------------------------- | :----------------------------------------------- | :--------------------- |
| [Rust RFC 1824 survey](https://rust-lang.github.io/rfcs/1824-crates.io-default-ranking.html)                       | Docs are #1 evaluation criterion (104 mentions)  | Strong (Rust-specific) |
| [Academic study (2024)](https://arxiv.org/html/2403.03819v1)                                                       | 91% of practitioners depend on docs for adoption | Strong                 |
| [GitHub survey (2017, n=5500)](https://opensourcesurvey.org/2017/)                                                 | 93% see poor docs as the top problem             | Strong                 |
| [Stack Overflow (2024)](https://survey.stackoverflow.co/2024/)                                                     | 84% use docs to learn; 90% use API/SDK docs      | Strong                 |
| [Library adoption research](https://journalofbigdata.springeropen.com/articles/10.1186/s40537-019-0201-8)          | Documentation builds trust for adoption          | Moderate               |
| [Crate evaluation guide](https://crates.community/article/How_to_choose_the_best_Rust_crate_for_your_project.html) | Docs listed as a top evaluation criterion        | Moderate               |
