---
title: "Changelog"
description: "Release-by-release list of Swiftide features, fixes and breaking changes, up to the version these docs describe."
sidebar:
  order: 3
tableOfContents:
  maxHeadingLevel: 2
---

This page lists the features, fixes and breaking changes of every Swiftide release up to {{swiftide_version}}, the version these docs describe.
One changelog covers every crate in the workspace; see [Versioning](/reference/crates/#versioning).
The list below is the repository's `CHANGELOG.md` at the commit these docs pin.
That commit can be ahead of the release.
Features marked unreleased in these docs are on swiftide master and do not appear in the list yet.
Entries marked **breaking** change the public API.
For the code changes each breaking release needs, see [Upgrading](/reference/upgrading/).

::include{src="@swiftide/CHANGELOG.md" strip="^(# Changelog|All notable changes to this project)" llms="false"}

Next: [Upgrading](/reference/upgrading/)
