import type { ReactNode } from "react";
import { GitlabListViewContext, useGitlabListViewSource } from "./useGitlabListView";

/** Keeps the GitLab list state (kind, hidden, view filter) alive across tab switches; kind and hidden are also remembered across restarts. */
export function GitlabListViewProvider({ children }: { children: ReactNode }) {
  const value = useGitlabListViewSource();
  return <GitlabListViewContext value={value}>{children}</GitlabListViewContext>;
}
