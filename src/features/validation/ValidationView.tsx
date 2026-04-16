import type { ValidationCase, ValidationResult } from '../../shared/types/clipsync';

type ValidationViewProps = {
  cases: ValidationCase[];
  onResultChange: (id: string, result: ValidationResult) => void;
  onNotesChange: (id: string, notes: string) => void;
  onExport: () => void;
  onReset: () => void;
};

export function ValidationView({
  cases,
  onResultChange,
  onNotesChange,
  onExport,
  onReset,
}: ValidationViewProps) {
  const passCount = cases.filter((item) => item.result === 'pass').length;
  const failCount = cases.filter((item) => item.result === 'fail').length;
  const pendingCount = cases.filter((item) => item.result === 'not-run').length;

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <div className="bg-gray-900 rounded-3xl p-10">
        <h2 className="text-3xl font-semibold mb-3">Validation Matrix</h2>
        <p className="text-gray-400 mb-6">
          Track step-11 verification in one place and export a JSON report for release readiness.
        </p>

        <div className="sync-stats-grid mb-6">
          <div>Pass: {passCount}</div>
          <div>Fail: {failCount}</div>
          <div>Not run: {pendingCount}</div>
          <div>Total: {cases.length}</div>
        </div>

        <div className="validation-actions-row">
          <button className="settings-save-btn" onClick={onExport}>Export Report</button>
          <button className="validation-reset-btn" onClick={onReset}>Reset Matrix</button>
        </div>
      </div>

      <div className="bg-gray-900 rounded-3xl p-8">
        {cases.map((item) => (
          <div key={item.id} className="validation-card">
            <div className="validation-card-head">
              <div>
                <div className="font-medium">{item.title}</div>
                <div className="text-gray-400 text-xs">{item.description}</div>
              </div>
              <div className="validation-chip-group">
                <button
                  className={`validation-chip ${item.result === 'pass' ? 'validation-chip-pass' : ''}`}
                  onClick={() => onResultChange(item.id, 'pass')}
                >
                  Pass
                </button>
                <button
                  className={`validation-chip ${item.result === 'fail' ? 'validation-chip-fail' : ''}`}
                  onClick={() => onResultChange(item.id, 'fail')}
                >
                  Fail
                </button>
                <button
                  className={`validation-chip ${item.result === 'not-run' ? 'validation-chip-pending' : ''}`}
                  onClick={() => onResultChange(item.id, 'not-run')}
                >
                  Not run
                </button>
              </div>
            </div>

            <textarea
              value={item.notes}
              onChange={(event) => onNotesChange(item.id, event.target.value)}
              className="settings-input validation-notes"
              placeholder="Add evidence, logs, and reproduction details"
            />
            <p className="settings-hint">
              Last run: {item.last_run_at ? new Date(item.last_run_at).toLocaleString() : 'never'}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}
