import type { ReactNode } from "react";
import "./SourceChip.css";

export type SourceKind = "gl" | "gh" | "my" | "mr" | "iss";

const LABELS: Record<SourceKind, string> = {
  gl: "GL",
  gh: "GH",
  my: "MY",
  mr: "MR",
  iss: "ISS",
};

export type SourceChipProps = {
  /** gl GitLab, gh GitHub, my own task, mr merge request, iss issue. */
  source: SourceKind;
  children?: ReactNode;
  title?: string;
  className?: string;
};

export function SourceChip({ source, children, title, className }: SourceChipProps) {
  return (
    <span
      className={["dn-chip", `dn-chip-${source}`, className].filter(Boolean).join(" ")}
      title={title}
    >
      {children ?? LABELS[source]}
    </span>
  );
}
