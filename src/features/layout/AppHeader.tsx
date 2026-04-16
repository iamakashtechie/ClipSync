import type { AppTab } from '../../shared/types/clipsync';

type AppHeaderProps = {
  currentTab: AppTab;
  onTabChange: (tab: AppTab) => void;
  devModeEnabled?: boolean;
};

export function AppHeader({ currentTab, onTabChange, devModeEnabled = false }: AppHeaderProps) {
  return (
    <div className="bg-gray-900 border-b border-gray-800 px-6 py-4 flex items-center justify-between">
      <div className="flex items-center gap-3">
        <span className="text-3xl" aria-hidden="true">📋</span>
        <h1 className="text-2xl font-bold">ClipSync</h1>
      </div>

      <div className="flex gap-1 bg-gray-800 rounded-xl p-1" role="tablist" aria-label="Main tabs">
        <button
          role="tab"
          aria-selected={currentTab === 'dashboard'}
          onClick={() => onTabChange('dashboard')}
          className={`px-6 py-2 rounded-xl font-medium ${currentTab === 'dashboard' ? 'bg-gray-900 text-white' : 'text-gray-400'}`}
        >
          Dashboard
        </button>
        {devModeEnabled && (
          <button
            role="tab"
            aria-selected={currentTab === 'validation'}
            onClick={() => onTabChange('validation')}
            className={`px-6 py-2 rounded-xl font-medium ${currentTab === 'validation' ? 'bg-gray-900 text-white' : 'text-gray-400'}`}
          >
            Validation
          </button>
        )}
        <button
          role="tab"
          aria-selected={currentTab === 'settings'}
          onClick={() => onTabChange('settings')}
          className={`px-6 py-2 rounded-xl font-medium ${currentTab === 'settings' ? 'bg-gray-900 text-white' : 'text-gray-400'}`}
        >
          Settings
        </button>
      </div>
    </div>
  );
}
