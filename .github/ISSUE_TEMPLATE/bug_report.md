---
name: Bug Report
about: Report a bug or unexpected behavior in rawk
title: '[BUG] '
labels: bug
assignees: ''
---

## Bug Description

A clear and concise description of what the bug is.

## AWK Program

```awk
# The AWK program that produces the bug
{ print $1 }
```

## Input Data

```
# The input data that triggers the bug
line1 data1
line2 data2
```

## Expected Behavior

What you expected to happen. If possible, show the output from `gawk` or another AWK implementation:

```
# Expected output
data1
data2
```

## Actual Behavior

What actually happened:

```
# Actual output from rawk
```

## Environment

- **rawk version**: `rawk --version`
- **Operating System**: [e.g., Ubuntu 22.04, macOS 14, Windows 11]
- **Rust version** (if building from source): `rustc --version`

## Additional Context

Add any other context about the problem here.

## Checklist

- [ ] I have searched existing issues to ensure this is not a duplicate
- [ ] I have tested with the latest version of rawk
- [ ] I have compared the behavior with gawk or another AWK implementation
