# Workspace ownership

Dagsverk uses workspaces to separate employment contexts. A workspace can represent an employer, a client, or personal work.

## Application scope

Application settings apply to every workspace and device record.

- Active workspace
- Theme preference
- Interface language
- Interface scale
- Default month view
- Update preference
- Database location, backup, and restore

Application settings must not contain employee, employer, pay, schedule, project, or balance data.

## Workspace scope

A workspace owns the rules and records for one work context.

### Identity

- Workspace name
- Workspace type: employment, contract, or personal
- Organization name, optional
- Worker name, optional
- Workspace color

The interface uses neutral labels. It shows organization and worker fields only when the selected workspace type needs them.

### Work rules

- Working weekdays
- Expected hours
- Default start, end, and lunch times
- Public holiday handling
- Overtime and unsocial-hours rules

### Compensation

- Salary type
- Salary or hourly rate
- Employment percentage
- Currency
- Tax settings
- Overtime compensation mode and rates

### Records

- Projects
- Work entries
- Month records
- Export settings

Deleting a workspace requires confirmation. The application must prevent deletion of the last workspace.

## Month scope

A month record owns values that can change between months.

- Opening time balance
- Expected-hours override
- Opening-balance edit state
- Started or finalized state, if Dagsverk adds that workflow

## Entry scope

A work entry owns values for one date in one workspace.

- Status
- Start and end time
- Lunch duration
- Scheduled-hours override
- Project
- Notes

## Interface structure

The Workspaces page manages identity, workspace type, and switching. Workspace settings contain Schedule, Compensation, Tax, Overtime, and Export sections. Application settings contain Appearance, Language, Updates, and Data sections.

The active workspace controls every ledger, calendar, report, project, balance, and calculation query. A workspace switch reloads these values as one operation.

## Migration rule

Existing Dagsverk data moves into one default employment workspace. The migration creates a safety backup before schema changes. It preserves all entries, month records, projects, and settings.
