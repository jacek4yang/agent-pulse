import { api, errorMessage } from "../lib/api";
import { useI18n } from "../lib/i18n";
import { formatDuration, formatTime } from "../lib/format";
import type { HistoryRecord } from "../lib/types";
import { Card, Empty } from "../components/ui";

export function History({ records, onChanged }: { records: HistoryRecord[]; onChanged: () => void }) {
  const { t, locale } = useI18n();
  return (
    <>
      <div className="page-header">
        <div>
          <h1>{t("historyTitle")}</h1>
          <div className="page-sub">{t("historySub")}</div>
        </div>
        <button
          className="btn small"
          onClick={async () => {
            await api.clearHistory().catch((e) => alert(errorMessage(e)));
            onChanged();
          }}
        >
          {t("clear")}
        </button>
      </div>
      <Card>
        {records.length === 0 ? (
          <Empty icon="🗒" title={t("noExecutions")} hint={t("noExecutionsHint")} />
        ) : (
          records.map((r) => <HistoryRow key={r.id} record={r} locale={locale} />)
        )}
      </Card>
    </>
  );
}

function HistoryRow({ record, locale }: { record: HistoryRecord; locale: string }) {
  const outcome = record.outcome;
  const ok = outcome.outcome === "success";
  return (
    <div className="history-row">
      <span className={`history-dot ${ok ? "ok" : "fail"}`} aria-hidden />
      <div className="history-main">
        <div className="history-task">{record.task_name}</div>
        <div className="history-err" style={{ color: ok ? "var(--text-tertiary)" : undefined }}>
          {ok
            ? record.target_description
            : `${outcome.error_code} — ${outcome.error_message}`}
        </div>
      </div>
      <div className="history-time">
        <div>{formatTime(record.started_at, locale)}</div>
        <div>{formatDuration(record.started_at, record.finished_at)}</div>
      </div>
    </div>
  );
}
