import { describe, it, expect, beforeEach } from 'vitest';
import { ElectronBridgeService } from './electron-bridge.service';
import { WorkEntry, WorkEntryStatus, Workspace, WorkspaceType } from './models';
import { matchingWeekdayOccurrence } from './app-state.service';

describe('Workspace Isolation & Multi-Tenancy', () => {
  let bridge: ElectronBridgeService;

  beforeEach(() => {
    bridge = new ElectronBridgeService();
  });

  it('should isolate entries between distinct workspaces', async () => {
    const ws1: Workspace = {
      id: 'ws-job-1',
      name: 'Primary Job',
      color: '#0B57D0',
      type: WorkspaceType.Employment,
      organizationName: 'Acme AB',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString()
    };

    const ws2: Workspace = {
      id: 'ws-job-2',
      name: 'Consulting',
      color: '#34A853',
      type: WorkspaceType.Contract,
      organizationName: 'Beta AB',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString()
    };

    await bridge.saveWorkspace(ws1);
    await bridge.saveWorkspace(ws2);

    const entry1: WorkEntry = {
      workspaceId: ws1.id,
      date: '2026-08-10',
      status: WorkEntryStatus.Worked,
      startTime: '08:00',
      endTime: '16:30',
      lunchMinutes: 30,
      projectName: 'Acme-Core',
      dayOffReason: null,
      notes: 'Full-time work',
      scheduledMinutesOverride: null,
      compTimeMinutes: 0,
    };

    const entry2: WorkEntry = {
      workspaceId: ws2.id,
      date: '2026-08-10',
      status: WorkEntryStatus.Worked,
      startTime: '18:00',
      endTime: '20:00',
      lunchMinutes: 0,
      projectName: 'Consulting-Proj',
      dayOffReason: null,
      notes: 'Evening gig',
      scheduledMinutesOverride: null,
      compTimeMinutes: 0,
    };

    await bridge.saveWorkEntry(entry1, ws1.id);
    await bridge.saveWorkEntry(entry2, ws2.id);

    const entriesWs1 = await bridge.getWorkEntries(2026, 8, ws1.id);
    const entriesWs2 = await bridge.getWorkEntries(2026, 8, ws2.id);

    expect(entriesWs1.length).toBe(1);
    expect(entriesWs1[0].projectName).toBe('Acme-Core');

    expect(entriesWs2.length).toBe(1);
    expect(entriesWs2[0].projectName).toBe('Consulting-Proj');
  });

  it('should isolate projects and settings per workspace', async () => {
    const wsA = 'ws-test-a';
    const wsB = 'ws-test-b';

    await bridge.saveProject({
      workspaceId: wsA,
      id: 'p-a',
      name: 'Alpha Project',
      color: '#0B57D0',
      isActive: true,
      isDefault: true
    }, wsA);

    await bridge.saveProject({
      workspaceId: wsB,
      id: 'p-b',
      name: 'Beta Project',
      color: '#EA4335',
      isActive: true,
      isDefault: true
    }, wsB);

    const projsA = await bridge.getProjects(wsA);
    const projsB = await bridge.getProjects(wsB);

    expect(projsA.some(p => p.name === 'Alpha Project')).toBe(true);
    expect(projsA.some(p => p.name === 'Beta Project')).toBe(false);

    expect(projsB.some(p => p.name === 'Beta Project')).toBe(true);
    expect(projsB.some(p => p.name === 'Alpha Project')).toBe(false);
  });

  it('matches copied entries by weekday occurrence', () => {
    expect(matchingWeekdayOccurrence('2026-06-01', 2026, 7)).toBe('2026-07-06');
    expect(matchingWeekdayOccurrence('2026-06-29', 2026, 2)).toBeNull();
  });
});
