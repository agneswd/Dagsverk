import { Component, EventEmitter, inject, Input, Output } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink, RouterLinkActive } from '@angular/router';
import { MatListModule } from '@angular/material/list';
import { MatIconModule } from '@angular/material/icon';
import { MatDividerModule } from '@angular/material/divider';
import { MatMenuModule } from '@angular/material/menu';
import { MatButtonModule } from '@angular/material/button';
import { MatRippleModule } from '@angular/material/core';
import { MatTooltipModule } from '@angular/material/tooltip';
import { MatDialog, MatDialogModule } from '@angular/material/dialog';
import { firstValueFrom } from 'rxjs';
import { AppStateService } from '../../core/app-state.service';
import { Workspace } from '../../core/models';
import { WorkspacesComponent } from '../../features/workspaces/workspaces.component';
import { RenameDialogComponent } from '../../shared/rename-dialog/rename-dialog.component';
import { ElectronBridgeService } from '../../core/electron-bridge.service';

interface NavItem {
  route: string;
  label: string;
  icon: string;
}

@Component({
  selector: 'app-sidebar',
  standalone: true,
  imports: [
    CommonModule,
    RouterLink,
    RouterLinkActive,
    MatListModule,
    MatIconModule,
    MatDividerModule,
    MatMenuModule,
    MatButtonModule,
    MatRippleModule,
    MatTooltipModule,
    MatDialogModule,
  ],
  templateUrl: './sidebar.component.html',
  styleUrls: ['./sidebar.component.scss'],
})
export class SidebarComponent {
  @Input() public collapsed = false;
  @Output() public collapsedChange = new EventEmitter<boolean>();
  public state = inject(AppStateService);
  public updates = inject(ElectronBridgeService);
  private dialog = inject(MatDialog);

  public navItems: NavItem[] = [
    { route: '/timesheet', label: 'Timesheet', icon: 'schedule' },
    { route: '/projects', label: 'Projects', icon: 'folder' },
  ];
  public settingsItem: NavItem = { route: '/settings', label: 'Settings', icon: 'settings' };

  public get showUpdateStatus(): boolean {
    return ['available', 'downloading', 'ready', 'error'].includes(
      this.updates.updateState().status,
    );
  }

  public get updateIcon(): string {
    switch (this.updates.updateState().status) {
      case 'ready':
        return 'check_circle';
      case 'error':
        return 'error';
      default:
        return 'download';
    }
  }

  public get updateLabel(): string {
    const update = this.updates.updateState();
    switch (update.status) {
      case 'ready':
        return 'Restart to update';
      case 'error':
        return 'Retry update';
      case 'downloading':
        return `Downloading update ${update.progress || 0}%`;
      default:
        return 'Update available';
    }
  }

  public onUpdateAction(): void {
    if (this.updates.updateState().status === 'ready') this.updates.restartToUpdate();
    else void this.updates.checkForUpdates();
  }

  public onSelectWorkspace(ws: Workspace): void {
    this.state.switchWorkspace(ws.id);
  }

  public onManageWorkspaces(): void {
    this.dialog.open(WorkspacesComponent, {
      width: '760px',
      maxWidth: 'calc(100vw - 32px)',
      maxHeight: 'calc(100vh - 32px)',
      panelClass: 'workspace-manager-dialog',
    });
  }

  public async onRenameWorkspace(): Promise<void> {
    const workspace = this.state.activeWorkspace();
    const name = await this.openRenameDialog('Rename workspace', workspace.name);
    if (name === undefined || name === workspace.name) return;
    await this.state.saveWorkspace({ ...workspace, name, updatedAt: new Date().toISOString() });
  }

  private openRenameDialog(title: string, initialName: string): Promise<string | undefined> {
    return firstValueFrom(
      this.dialog
        .open(RenameDialogComponent, {
          width: '440px',
          data: { title, label: 'New name', initialName },
        })
        .afterClosed(),
    );
  }

  public toggleCollapsed(): void {
    this.collapsedChange.emit(!this.collapsed);
  }
}
