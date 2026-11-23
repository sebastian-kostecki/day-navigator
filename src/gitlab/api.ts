import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { GitlabSnapshot } from "./types";

export function getGitlabSnapshot(): Promise<GitlabSnapshot> {
  return invoke<GitlabSnapshot>("gitlab_snapshot");
}

/** Wakes the refresh thread; the result arrives as a `gitlab-changed` event. */
export function refreshGitlab(): Promise<void> {
  return invoke<void>("refresh_gitlab");
}

/** Opens an item in the default browser. */
export function openGitlabItem(url: string): Promise<void> {
  return openUrl(url);
}
