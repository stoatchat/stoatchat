---
sidebar_position: 2
---

# Contribution Guide

This is the contribution guide for developers wanting to help out with Stoat.

## Repository Lifecycle

### Making Commits

- Sign-off your commits ([Git flag](https://git-scm.com/docs/git-commit#Documentation/git-commit.txt---signoff)), [read here about DCO obligations](https://developercertificate.org/).
- Sign commits where possible, [learn more about that here](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits).
- Prefer to use [Conventional Commit style](https://www.conventionalcommits.org/en/v1.0.0-beta.2/).
- If present, e.g. `prettier`, `cargo fmt`, use the formatter.
- Try to keep each PR bound to a single feature or change, multiple bug fixes may be fine in some cases.
  This is to avoid your PR getting stuck due to parts of it having conflicts or other issues.

### Merging Pull Requests

All PR titles must use use [Conventional Commit style](https://www.conventionalcommits.org/en/v1.0.0-beta.2/) and will be squash merged!

## What can I help with?

Stuff is currently being moved around, for the mean time, come ask in the development server: https://stt.gg/API

Also typically `help wanted` labels are available on repo issues!

<!-- The main project board can serve as a helpful starting point:

1. If you are new to the code base or are looking for issues we really need help with, look at ["What can I help with?"](https://github.com/orgs/projects/3/views/11)
2. Issue Board ["Free Issues"](https://github.com/orgs/projects/3/views/1): issues that anyone can pick up and are generally free to work on
3. Issue Board ["Todo"](https://github.com/orgs/projects/3/views/1): these are issues that are probably fine to pick up, but please ask first since a lot of these tend to be complicated and potentially already planned
4. Working on new issues and fixes: ideally you should run new features by us, most fixes are probably going to be alright though, we wouldn't want to reject any PRs that we don't deem suitable after work has already been done. If it's a fix, make sure to make an issue for it first, if it's a new feature, it may be better suited in [Feature Suggestions](https://github.com/discussions/categories/feature-suggestions)

Any issues marked with "Future Work" or with a milestone greater than the current milestone are out of bounds and should not be worked on since it's likely that the team already has a plan in place, any work you may do may conflict with prior ideas, and your work may potentially be rejected if it does fit the criteria exactly. In general, these issues are just postponed to reduce long term technical debt, i.e. allow current issues to be handled. -->

## Project Guidance

Please read the additional relevant guidance on:

- [Developing for Backend](https://github.com/stoatchat/backend?tab=readme-ov-file#development-guide) (contrib guide TBA)
- [Contributing to Frontend](https://stoatchat.github.io/for-web/contribution-guide.html)
- [Contributing to Android](https://stoatchat.github.io/for-android/contributing/guidelines/)
