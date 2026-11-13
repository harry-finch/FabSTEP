<script lang="ts">
  // Mise en page de référence : barre d'application, panneau de réglage
  // (gauche), espace de travail (centre), inspecteur (droite), barre d'état.
  // Exemple de contenu pour montrer chaque composant ; à remplacer.
  import AppBar from './components/AppBar.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import Section from './components/Section.svelte';
  import NumField from './components/NumField.svelte';
  import Field from './components/Field.svelte';
  import Segmented from './components/Segmented.svelte';
  import Disclosure from './components/Disclosure.svelte';
  import Dialog from './components/Dialog.svelte';
  import Alerts from './components/Alerts.svelte';
  import Tag from './components/Tag.svelte';
  import TaskList from './components/TaskList.svelte';
  import ContextCard from './components/ContextCard.svelte';
  import ProposalCard from './components/ProposalCard.svelte';
  import RecordsTable from './components/RecordsTable.svelte';
  import AssistantPanel from './components/AssistantPanel.svelte';
  import { Calculator, FlaskConical } from '@lucide/svelte';
  import { ui } from './lib/ui.svelte';
  import { t } from './lib/i18n.svelte';

  let name = $state('');
  let width = $state(120);
  let margin = $state(4);
  let mode = $state<'a' | 'b' | 'c'>('a');
  let shape = $state<'rect' | 'disc'>('rect');
  let proposal = $state(true);
  let messages = $state<{ id: string; author: string; text: string }[]>([]);

  function keydown(e: KeyboardEvent) {
    const el = e.target as HTMLElement | null;
    if (el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable)) return;
    if (e.key === '?') ui.help = true;
  }
</script>

<svelte:window onkeydown={keydown} />

<div class="app">
  <AppBar bind:name />
  <div class="main">
    <aside class="panel left">
      <Section id="plate" title="Plaque" step={1}>
        <NumField label="Largeur" value={width} min={0} onchange={(v) => (width = v)} />
        <NumField label="Marge" hint="Bande sans perforation le long du bord." value={margin} min={0} onchange={(v) => (margin = v)} invalid={margin * 2 >= width} />
        <Field label="Contour">
          <Segmented label="Contour" value={shape} onchange={(v) => (shape = v)} options={[{ id: 'rect', label: 'Rectangle' }, { id: 'disc', label: 'Disque' }]} />
        </Field>
        <!-- Trois options ou plus : sous-titre puis sélecteur sur toute la largeur. -->
        <p class="sub">Bords</p>
        <Segmented label="Bords" value={mode} onchange={(v) => (mode = v)} options={[{ id: 'a', label: 'Centre' }, { id: 'b', label: 'Entiers' }, { id: 'c', label: 'Découper' }]} />
        <Disclosure id="plate.advanced" label="Avancé">
          <NumField label="Angle" kind="angle" value={0} onchange={() => {}} />
        </Disclosure>
      </Section>
    </aside>
    <main class="workspace">
      <div class="examples">
        <h2>Exemples de motifs</h2>
        <TaskList label="Tâches" tasks={[{ id: '1', title: 'Vérification', state: 'En cours', detail: 'Contrôle des données', progress: 65 }]} />
        <ContextCard title="Source" excerpt="Paramètres de la machine." source="Document local" />
        {#if proposal}<ProposalCard title="Proposition" description="Ajuster la marge à 5 mm." applyLabel="Appliquer" rejectLabel="Rejeter" onapply={() => { margin = 5; proposal = false; }} onreject={() => { proposal = false; }} />{/if}
        <RecordsTable columns={['Pièce', 'État']} rows={[{ id: '1', cells: ['Plaque', 'Prêt'], status: 'Prêt' }, { id: '2', cells: ['Support', 'À vérifier'], status: 'À vérifier' }]} searchLabel="Rechercher" filterLabel="État" allLabel="Tous" emptyLabel="Aucune donnée." noResultsLabel="Aucun résultat." countLabel="résultat(s)" />
        <AssistantPanel label="Assistance" {messages} inputLabel="Message" sendLabel="Envoyer" busyLabel="En cours" onsubmit={(text) => { messages = [...messages, { id: String(messages.length), author: 'Vous', text }]; }} />
      </div>
    </main>
    <aside class="panel right">
      <Section id="metrics" title="Mesures">
        <dl class="metrics">
          <dt>Taux d’ouverture</dt>
          <dd class="strong tabular">35,3 %</dd>
          <dt>Origine</dt>
          <dd><Tag label="Calculé" icon={Calculator} /> <Tag label="Testé" icon={FlaskConical} tone="info" /></dd>
        </dl>
      </Section>
      <Section id="issues" title="Messages">
        <Alerts
          alerts={[
            { code: 'a', severity: 'warning', message: '3 nervures plus fines que le minimum.' },
            { code: 'b', severity: 'info', message: '12 perforations retirées près du bord.' },
          ]}
        />
      </Section>
    </aside>
  </div>
  <StatusBar version="0.1.0" />
</div>

{#if ui.help}
  <Dialog title={t('help.title')} onclose={() => (ui.help = false)}>
    <h3>{t('help.shortcuts')}</h3>
    <dl class="keys">
      <dt><kbd>?</kbd></dt>
      <dd>{t('help.keys.help')}</dd>
      <dt><kbd>⌘Z / ⇧⌘Z</kbd></dt>
      <dd>{t('help.keys.undo')}</dd>
    </dl>
  </Dialog>
{/if}

<div class="toasts" aria-live="polite">
  {#each ui.toasts as toast (toast.id)}
    <div class="toast">{toast.text}</div>
  {/each}
</div>

<style>
  .app {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: 100vh;
    min-width: 0;
  }

  .main {
    display: grid;
    grid-template-columns: 296px minmax(0, 1fr) 296px;
    min-height: 0;
  }

  @media (max-width: 1100px) {
    .main {
      grid-template-columns: 264px minmax(0, 1fr) 264px;
    }
  }

  .panel {
    background: var(--bg-panel);
    overflow-y: auto;
    min-height: 0;
  }

  .left {
    border-right: 1px solid var(--border);
  }

  .right {
    border-left: 1px solid var(--border);
  }

  .workspace {
    display: grid;
    align-content: start;
    overflow-y: auto;
    padding: var(--s4);
    min-width: 0;
    background: var(--bg-workspace);
  }

  .examples { display: grid; gap: var(--s3); width: min(100%, 560px); margin: 0 auto; }
  .examples h2 { font-size: var(--fs-lg); margin: 0; }

  .sub {
    margin: var(--s1) 0 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .metrics {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 6px var(--s2);
    margin: 0;
  }

  .metrics dt {
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .metrics dd {
    margin: 0;
    text-align: right;
  }

  .metrics dd.strong {
    font-weight: 600;
    font-size: var(--fs-lg);
  }

  .keys {
    display: grid;
    grid-template-columns: 120px 1fr;
    gap: 6px var(--s3);
    margin: 0;
  }

  .keys dd {
    margin: 0;
  }

  kbd {
    display: inline-block;
    padding: 1px 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--bg-control);
    font: 500 var(--fs-sm) var(--font);
    color: var(--text-primary);
  }

  .toasts {
    position: fixed;
    bottom: 36px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: 6px;
    z-index: 900;
    pointer-events: none;
  }

  .toast {
    padding: 8px 14px;
    border-radius: var(--radius-lg);
    background: var(--tooltip-bg);
    color: var(--tooltip-text);
    font-size: var(--fs-sm);
    box-shadow: var(--shadow-pop);
    animation: rise 160ms ease-out;
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
</style>
