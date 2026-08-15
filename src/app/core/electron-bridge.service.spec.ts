import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ElectronBridgeService } from './electron-bridge.service';
import { UpdateState } from './models';

describe('ElectronBridgeService', () => {
  let updateListener: (state: UpdateState) => void;
  const setZoomFactor = vi.fn();

  beforeEach(() => {
    setZoomFactor.mockClear();
    Object.defineProperty(window, 'electronAPI', {
      configurable: true,
      value: {
        getUpdateState: vi.fn().mockResolvedValue({ status: 'idle', currentVersion: '0.1.1' }),
        onUpdateState: vi.fn((listener: (state: UpdateState) => void) => {
          updateListener = listener;
          return () => undefined;
        }),
        setZoomFactor,
      },
    });
  });

  afterEach(() => {
    delete window.electronAPI;
  });

  it('uses native zoom and shares update events', () => {
    const bridge = new ElectronBridgeService();

    bridge.setZoomFactor(1.1);
    updateListener({ status: 'ready', currentVersion: '0.1.1', availableVersion: '0.1.2' });

    expect(setZoomFactor).toHaveBeenCalledWith(1.1);
    expect(bridge.updateState().status).toBe('ready');
  });
});
