import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterLink, RouterLinkActive } from '@angular/router';
import { MatListModule } from '@angular/material/list';
import { MatIconModule } from '@angular/material/icon';
import { MatDividerModule } from '@angular/material/divider';
import { MatMenuModule } from '@angular/material/menu';
import { MatButtonModule } from '@angular/material/button';
import { MatRippleModule } from '@angular/material/core';
import { AppStateService } from '../../core/app-state.service';
import { Workspace } from '../../core/models';

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
    MatRippleModule
  ],
  templateUrl: './sidebar.component.html',
  styleUrls: ['./sidebar.component.scss']
})
export class SidebarComponent {
  public state = inject(AppStateService);
  private router = inject(Router);

  public navItems: NavItem[] = [
    { route: '/timesheet', label: 'Timesheet', icon: 'schedule' },
    { route: '/workspaces', label: 'Workspaces', icon: 'corporate_fare' },
    { route: '/projects', label: 'Projects', icon: 'folder' },
    { route: '/settings', label: 'Settings', icon: 'settings' },
    { route: '/backups', label: 'Data & backups', icon: 'backup' }
  ];

  public onSelectWorkspace(ws: Workspace): void {
    this.state.switchWorkspace(ws.id);
  }

  public onManageWorkspaces(): void {
    this.router.navigate(['/workspaces']);
  }
}
