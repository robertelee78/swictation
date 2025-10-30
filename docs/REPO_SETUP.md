# Repository Setup Guide

## Overview

Swictation uses a **two-repository structure** to separate code from environment configuration:

1. **`swictation`** (public) - Voice dictation system code
2. **`swictation-env`** (private) - Claude Flow environment configuration

## Structure

```
/opt/
├── swictation/              # Main repository
│   ├── .claude -> ../swictation-env/.claude  (symlink)
│   ├── src/
│   ├── tests/
│   └── ...
└── swictation-env/         # Environment repository (private)
    ├── .claude/
    │   ├── agents/         # 54 AI agent definitions
    │   ├── commands/       # Custom slash commands
    │   ├── skills/         # Reusable skill modules
    │   └── helpers/        # Automation scripts
    └── README.md
