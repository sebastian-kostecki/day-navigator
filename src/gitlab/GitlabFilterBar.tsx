import { Button, Chip, plural } from "../components";
import { useSettingsDialog } from "../settings/useSettingsDialog";
import type { FilterRule, GitlabFilterValues } from "./types";

function ruleHasValues(rule: FilterRule): boolean {
  return rule.include.length > 0 || rule.exclude.length > 0;
}

/** True when the filter has any entry or the closed section is on. */
function hasFilterValues(filter: GitlabFilterValues): boolean {
  return (
    ruleHasValues(filter.projects) ||
    ruleHasValues(filter.labels) ||
    ruleHasValues(filter.milestones) ||
    filter.closedDays !== null
  );
}

function FilterDimension({ label, rule }: { label: string; rule: FilterRule }) {
  if (!ruleHasValues(rule)) return null;
  return (
    <span className="gitlab-filter-group">
      <span className="meta">{label}</span>
      {rule.include.map((value, i) => (
        <span key={`in:${i}:${value}`} title={value} className="gitlab-filter-chip">
          <Chip>{value}</Chip>
        </span>
      ))}
      {rule.exclude.length > 0 && <span className="meta">bez:</span>}
      {rule.exclude.map((value, i) => (
        <span key={`ex:${i}:${value}`} title={value} className="gitlab-filter-chip">
          <Chip tone="faint">{value}</Chip>
        </span>
      ))}
    </span>
  );
}

/** The active `gitlabFilter` values above the list, with a way to change them. */
export function GitlabFilterBar({ filter }: { filter: GitlabFilterValues }) {
  const { openAt } = useSettingsDialog();

  if (!hasFilterValues(filter)) return null;

  return (
    <div className="gitlab-filter" role="region" aria-label="Aktywny filtr">
      <div className="gitlab-filter-bar">
        <span className="meta">Filtr z ustawień</span>
        <FilterDimension label="Projekty" rule={filter.projects} />
        <FilterDimension label="Etykiety" rule={filter.labels} />
        <FilterDimension label="Kamienie milowe" rule={filter.milestones} />
        {filter.closedDays !== null && (
          <span className="meta">
            Zamknięte: {filter.closedDays} {plural(filter.closedDays, "dzień", "dni", "dni")}
          </span>
        )}
        <span className="dn-spacer" />
        <Button variant="quiet" size="sm" icon="settings" onClick={() => openAt("gitlab")}>
          Ustawienia
        </Button>
      </div>
    </div>
  );
}
