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
import { AppStateService } from '../../core/app-state.service';
import { Workspace } from '../../core/models';
import { WorkspacesComponent } from '../../features/workspaces/workspaces.component';

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
  private dialog = inject(MatDialog);

  public navItems: NavItem[] = [
    { route: '/timesheet', label: 'Timesheet', icon: 'schedule' },
    { route: '/projects', label: 'Projects', icon: 'folder' },
    { route: '/settings', label: 'Settings', icon: 'settings' },
    { route: '/backups', label: 'Data & backups', icon: 'backup' },
  ];

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

  public toggleCollapsed(): void {
    this.collapsedChange.emit(!this.collapsed);
  }
}
