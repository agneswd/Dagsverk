import { Injectable, signal } from '@angular/core';
import { LanguagePreference } from './models';
import { ENGLISH_RESOURCES } from './i18n/en';
import { SWEDISH_RESOURCES } from './i18n/sv';

@Injectable({ providedIn: 'root' })
export class LocalizationService {
  public language = signal<'en' | 'sv'>('en');
  private observer?: MutationObserver;
  private applying = false;
  private textSources = new WeakMap<Text, string>();
  private attributeSources = new WeakMap<Element, Map<string, string>>();

  public constructor() {
    if (typeof document !== 'undefined') {
      queueMicrotask(() => this.start());
    }
  }

  public setPreference(preference: LanguagePreference): void {
    const language =
      preference === LanguagePreference.Swedish ||
      (preference === LanguagePreference.System && this.systemUsesSwedish())
        ? 'sv'
        : 'en';
    this.language.set(language);
    if (typeof document !== 'undefined') {
      document.documentElement.lang = language;
      this.translateTree(document.body);
    }
  }

  public t(source: string): string {
    return (this.language() === 'sv' ? SWEDISH_RESOURCES : ENGLISH_RESOURCES)[source] || source;
  }

  private systemUsesSwedish(): boolean {
    return typeof navigator !== 'undefined' && navigator.language.toLowerCase().startsWith('sv');
  }

  private start(): void {
    if (!document.body || this.observer) return;
    this.translateTree(document.body);
    this.observer = new MutationObserver((records) => {
      if (this.applying) return;
      for (const record of records) {
        if (record.type === 'characterData' && record.target instanceof Text) {
          const previousSource = this.textSources.get(record.target);
          if (previousSource && record.target.data === this.translateDynamic(previousSource))
            continue;
          this.textSources.set(record.target, record.target.data);
          this.translateText(record.target);
        } else {
          record.addedNodes.forEach((node) => this.translateTree(node));
        }
      }
    });
    this.observer.observe(document.body, { childList: true, subtree: true, characterData: true });
  }

  private translateTree(root: Node): void {
    if (!root) return;
    this.applying = true;
    try {
      if (root instanceof Text) this.translateText(root);
      if (root instanceof Element) this.translateElement(root);
      const walker = document.createTreeWalker(
        root,
        NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT,
      );
      let node: Node | null;
      while ((node = walker.nextNode())) {
        if (node instanceof Text) this.translateText(node);
        else if (node instanceof Element) this.translateElement(node);
      }
    } finally {
      this.applying = false;
    }
  }

  private translateText(node: Text): void {
    const source = this.textSources.get(node) ?? node.data;
    this.textSources.set(node, source);
    const trimmed = source.trim();
    if (!trimmed) return;
    const translated = this.translateDynamic(trimmed);
    const next = source.replace(trimmed, translated);
    if (node.data !== next) node.data = next;
  }

  private translateElement(element: Element): void {
    const attributes = ['aria-label', 'placeholder', 'title'];
    let sources = this.attributeSources.get(element);
    if (!sources) {
      sources = new Map();
      this.attributeSources.set(element, sources);
    }
    for (const name of attributes) {
      const value = element.getAttribute(name);
      if (!value) continue;
      const source = sources.get(name) ?? value;
      sources.set(name, source);
      element.setAttribute(name, this.translateDynamic(source));
    }
  }

  private translateDynamic(source: string): string {
    const direct = this.t(source);
    if (direct !== source) return direct;
    const patterns: Array<[RegExp, (match: RegExpMatchArray) => string]> = [
      [/^Catch Up \((\d+)\)$/, (match) => `Kom ikapp (${match[1]})`],
      [/^Available workspaces \((\d+)\)$/, (match) => `Tillgängliga arbetsytor (${match[1]})`],
      [/^Active projects \((\d+)\)$/, (match) => `Aktiva projekt (${match[1]})`],
      [/^Catch up - (\d+) of (\d+)$/, (match) => `Kom ikapp - ${match[1]} av ${match[2]}`],
      [/^Gross: (.+)$/, (match) => `Brutto: ${match[1]}`],
      [/^Add entry for (.+)$/, (match) => `Lägg till post för ${match[1]}`],
      [/^Archive (.+)$/, (match) => `Arkivera ${match[1]}`],
      [/^(.+) project color$/, (match) => `${this.t(match[1])} som projektfärg`],
      [/^Edit (.+)$/, (match) => `Redigera ${match[1]}`],
      [/^Change (.+) color$/, (match) => `Ändra färg för ${match[1]}`],
      [/^Downloading update (\d+)%$/, (match) => `Hämtar uppdatering ${match[1]} %`],
      [
        /^Dagsverk (.+) is ready to install\.$/,
        (match) => `Dagsverk ${match[1]} är redo att installeras.`,
      ],
      [
        /^Set the balance carried into (.+)\.$/,
        (match) => `Ange saldot som förs över till ${match[1]}.`,
      ],
      [/^Downloading version (.+)\.\.\.$/, (match) => `Hämtar version ${match[1]}...`],
      [/^Workspace "(.+)" created$/, (match) => `Arbetsytan "${match[1]}" skapades`],
      [/^Workspace "(.+)" deleted$/, (match) => `Arbetsytan "${match[1]}" togs bort`],
      [/^Backup saved: (.+)$/, (match) => `Säkerhetskopia sparad: ${match[1]}`],
      [/^Backup failed: (.+)$/, (match) => `Säkerhetskopiering misslyckades: ${match[1]}`],
      [/^Restore failed: (.+)$/, (match) => `Återställning misslyckades: ${match[1]}`],
      [/^Import failed: (.+)$/, (match) => `Import misslyckades: ${match[1]}`],
      [
        /^Imported (\d+) entries into (.+)$/,
        (match) => `Importerade ${match[1]} poster till ${match[2]}`,
      ],
      [
        /^Delete "(.+)" and all its entries, projects, and settings\?$/,
        (match) => `Ta bort "${match[1]}" och alla dess poster, projekt och inställningar?`,
      ],
      [
        /^Delete "(.+)"\? Existing time entries keep the project name\.$/,
        (match) => `Ta bort "${match[1]}"? Befintliga tidsposter behåller projektnamnet.`,
      ],
      [
        /^Replace the current database with (.+)\? Dagsverk will create a safety backup first\.$/,
        (match) =>
          `Ersätt den aktuella databasen med ${match[1]}? Dagsverk skapar först en säkerhetskopia.`,
      ],
      [
        /^(.+) saved entries and (.+) worked hours\.$/,
        (match) => `${match[1]} sparade poster och ${match[2]} arbetstimmar.`,
      ],
      [
        /^(\d+) past workdays are still unlogged\.$/,
        (match) => `${match[1]} arbetsdagar har fortfarande ingen registrerad tid.`,
      ],
      [/^Set (.+) as default project$/, (match) => `Ange ${match[1]} som standardprojekt`],
      [/^Activate (.+)$/, (match) => `Aktivera ${match[1]}`],
      [/^Delete (.+)$/, (match) => `Ta bort ${match[1]}`],
    ];
    if (this.language() === 'sv') {
      for (const [pattern, replace] of patterns) {
        const match = source.match(pattern);
        if (match) return replace(match);
      }
    }
    return source;
  }
}
