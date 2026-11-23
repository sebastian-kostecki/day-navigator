import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { GithubSnapshot } from "./types";

export function getGithubSnapshot(): Promise<GithubSnapshot> {
  return invoke<GithubSnapshot>("github_snapshot");
}

/** Wakes the refresh thread; the result arrives as a `github-changed` event. */
export function refreshGithub(): Promise<void> {
  return invoke<void>("refresh_github");
}

/** Opens an issue in the default browser. */
export function openGithubIssue(url: string): Promise<void> {
  return openUrl(url);
}
