---
change_id: gitlab-new-and-hidden-date
title: GitLab — flaga NOWE i data ukrycia
status: impl_reviewed
created: 2026-11-24
updated: 2026-11-24
---

## Notes

Slice S-02 z milestone'u M-2 (`konsola-gaps-and-cleanup`), GitHub issue #5, PRD FR-023. W „Praca (GitLab)” pozycje pierwszy raz widziane po ostatniej wizycie w widoku mają flagę NOWE, a ukryte wiersze pokazują datę ukrycia („ukryte 18.11”). Rust liczy obie informacje z istniejących `seen_assignments.first_seen_ms` i `hidden_issues.hidden_at_ms` oraz z nowego znacznika wizyty per konto (migracja v9: nieprzezroczysty klucz konta i jeden czas, bez treści z pracy).
