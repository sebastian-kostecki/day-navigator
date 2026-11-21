---
starter_id: tauri
package_manager: cargo
project_name: day-navigator
hints:
  language_family: rust
  team_size: solo
  deployment_target: github-releases
  ci_provider: github-actions
  ci_default_flow: auto-deploy-on-merge
  bootstrapper_confidence: verified
  path_taken: standard
  quality_override: false
  self_check_answers: null
  has_auth: false
  has_payments: false
  has_realtime: false
  has_ai: false
  has_background_jobs: true
---

## Why this stack

A solo developer building a personal Windows desktop app over 12 after-hours weeks, at small scale, with no in-app auth, payments, realtime or AI, but with substantial background work: interval sync of assigned issues, morning and work-time reminders, break intervals and a scheduled distraction blocker. Tauri is the recommended default for desktop in the Rust family and matches the user's own earlier decision for a local desktop app over CLI tools and files. It clears all four agent-friendly gates and its scaffolding is verified end to end. The Rust side covers the OS integrations the PRD leans on (tray, autostart, global shortcut, notifications, scheduled tasks), while the UI uses the card's default React + TypeScript template, chosen over the Vue setup of an earlier prototype. Installers are published as GitHub Releases by GitHub Actions when a merge to main bumps the version in src-tauri/Cargo.toml; every other merge only runs CI. Note for scaffolding: the React template needs a JavaScript package manager for the frontend (npm), even though the card lists cargo for the Rust side.
