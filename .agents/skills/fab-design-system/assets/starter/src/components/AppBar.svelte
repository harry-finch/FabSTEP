<script lang="ts">
  // Barre d'application : identité (logo, nom, sous-titre), menu Fichier et
  // encart du document, réglages d'affichage (unités, langue, thème), aide,
  // et l'unique action principale de l'application (ici : Exporter).
  import { ChevronDown, CircleHelp, Download, FileText, FilePlus, FolderOpen, Monitor, Moon, Save, Sun } from '@lucide/svelte';
  import Popover from './Popover.svelte';
  import { theme, THEMES } from '../lib/theme.svelte';
  import { ui } from '../lib/ui.svelte';
  import { tip } from '../lib/tip';
  import { i18n, LOCALES, t } from '../lib/i18n.svelte';
  import { APP_NAME } from '../lib/app';

  let { name = $bindable('') }: { name?: string } = $props();

  // Texte en cours de saisie : l'encart suit sa longueur (voir `.sizer`).
  let draft = $state<string | null>(null);
  const shown = $derived(draft ?? name);
</script>

<header class="bar">
  <div class="brand">
    <img class="logo" src="/logo.svg" alt="" width="24" height="24" />
    <span class="name">{APP_NAME}</span>
    <span class="tag">{t('app.tagline')}</span>
  </div>

  <div class="project">
    <Popover label={t('file.menu')} width={260}>
      {#snippet trigger({ open, toggle })}
        <button class="fbtn" class:open type="button" onclick={toggle} aria-haspopup="menu" aria-expanded={open}>
          {t('file.menu')}
          <ChevronDown size={13} />
        </button>
      {/snippet}
      {#snippet content({ close })}
        <button class="row" type="button" onclick={close}><FilePlus size={14} /><span class="rname">{t('file.new')}</span></button>
        <div class="divider"></div>
        <button class="row" type="button" onclick={close}><FolderOpen size={14} /><span class="rname">{t('file.open')}</span><span class="kbd">⌘O</span></button>
        <button class="row" type="button" onclick={close}><Save size={14} /><span class="rname">{t('file.save')}</span><span class="kbd">⌘S</span></button>
      {/snippet}
    </Popover>
    <!-- Encart du document : icône, libellé discret, nom en gras modifiable, largeur au texte. -->
    <label class="pbox" use:tip={t('file.nameHint')}>
      <FileText size={14} />
      <span class="plabel">{t('file.project')}</span>
      <span class="pfield">
        <span class="sizer" aria-hidden="true">{shown || t('file.untitled')}</span>
        <input
          class="pname"
          type="text"
          size="1"
          placeholder={t('file.untitled')}
          aria-label={t('file.name')}
          value={name}
          oninput={(e) => (draft = (e.currentTarget as HTMLInputElement).value)}
          onchange={(e) => {
            name = (e.currentTarget as HTMLInputElement).value;
            draft = null;
          }}
          onkeydown={(e) => e.key === 'Enter' && (e.currentTarget as HTMLInputElement).blur()}
        />
      </span>
    </label>
  </div>

  <div class="right">
    <div class="units" role="radiogroup" aria-label={t('app.units')}>
      {#each [['mm', 'mm'], ['in', t('app.inches')]] as [id, label] (id)}
        <button type="button" role="radio" aria-checked={ui.units === id} class:on={ui.units === id} onclick={() => (ui.units = id as 'mm' | 'in')}>{label}</button>
      {/each}
    </div>
    <div class="units" role="radiogroup" aria-label={t('app.language')}>
      {#each LOCALES as l (l.id)}
        <button type="button" role="radio" aria-checked={i18n.locale === l.id} aria-label={l.label} class:on={i18n.locale === l.id} onclick={() => i18n.set(l.id)}>{l.id.toUpperCase()}</button>
      {/each}
    </div>
    <div class="units theme" role="radiogroup" aria-label={t('app.theme')}>
      {#each THEMES as th (th)}
        {@const Icon = { system: Monitor, light: Sun, dark: Moon }[th]}
        <button type="button" role="radio" aria-checked={theme.choice === th} aria-label={t(`app.themes.${th}`)} class:on={theme.choice === th} use:tip={t(`app.themes.${th}`)} onclick={() => theme.set(th)}><Icon size={12} /></button>
      {/each}
    </div>
    <button class="btn quiet icon" type="button" use:tip={`${t('app.help')} (?)`} aria-label={t('app.help')} onclick={() => (ui.help = true)}>
      <CircleHelp size={16} />
    </button>
    <!-- Une seule action principale (bouton bleu) dans la barre. -->
    <button class="btn primary" type="button">
      <Download size={14} />
      {t('export.button')}
      <ChevronDown size={13} />
    </button>
  </div>
</header>

<style>
  .bar {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--s4);
    height: 42px;
    padding: 0 var(--s3);
    background: var(--bg-chrome);
    border-bottom: 1px solid var(--border);
  }

  /* Liseré d'identité : 2 px en haut de la fenêtre, seul grand usage de la couleur d'identité. */
  .bar::before {
    content: '';
    position: absolute;
    inset: 0 0 auto 0;
    height: 2px;
    background: var(--identity);
    opacity: 0.85;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  .logo {
    width: 24px;
    height: 24px;
    display: block;
  }

  .name {
    font-weight: 700;
    font-size: var(--fs-lg);
    letter-spacing: -0.01em;
  }

  .tag {
    color: var(--text-muted);
    font-size: var(--fs-sm);
    white-space: nowrap;
  }

  @media (max-width: 1100px) {
    .tag {
      display: none;
    }
  }

  .project {
    display: flex;
    align-items: center;
    gap: var(--s1);
    min-width: 0;
  }

  .project :global(.pop) {
    width: auto;
  }

  .fbtn {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding: 0 var(--s2);
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-size: var(--fs-md);
    white-space: nowrap;
  }

  .fbtn:hover,
  .fbtn.open {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .pbox {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--s2);
    min-width: 0;
    max-width: 460px;
    height: var(--control-h);
    padding: 0 var(--s2);
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    cursor: text;
  }

  .pbox:hover {
    border-color: var(--border-strong);
  }

  .pbox:focus-within {
    border-color: var(--accent);
  }

  .plabel {
    font-size: var(--fs-sm);
    white-space: nowrap;
  }

  /* Champ à la largeur de son texte : le texte invisible du `.sizer` et le
     champ partagent la même case de grille. */
  .pfield {
    display: inline-grid;
    min-width: 0;
    height: 100%;
    align-items: center;
  }

  .sizer,
  .pname {
    grid-area: 1 / 1;
    font: inherit;
    font-weight: 600;
    white-space: pre;
  }

  .sizer {
    visibility: hidden;
    min-width: 40px;
    padding-right: 2px;
    overflow: hidden;
  }

  .pname {
    width: 100%;
    min-width: 0;
    height: 100%;
    padding: 0;
    background: transparent;
    border: 0;
    outline: none;
    color: var(--text-primary);
    text-overflow: ellipsis;
  }

  .pname::placeholder {
    color: var(--text-muted);
  }

  @media (max-width: 1100px) {
    .pbox {
      max-width: 280px;
    }
  }

  /* Lignes de menu : icône 14 px, nom, raccourci ou description en muted. */
  .row {
    display: flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
    padding: 6px var(--s2);
    background: none;
    border: 0;
    border-radius: var(--radius);
    text-align: left;
    color: var(--text-secondary);
  }

  .row:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .rname {
    flex: 1;
    color: var(--text-primary);
    font-weight: 500;
  }

  .kbd {
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }

  .divider {
    height: 1px;
    margin: var(--s1) 0;
    background: var(--divider);
  }

  .right {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  /* Mini-sélecteurs de la barre (24 px) : même dessin que Segmented, plus bas. */
  .units {
    display: flex;
    height: 24px;
    padding: 2px;
    gap: 2px;
    background: var(--segment-track);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .units button {
    padding: 0 var(--s2);
    border: 0;
    border-radius: 3px;
    background: none;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .units button.on {
    background: var(--bg-elevated);
    color: var(--text-primary);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .theme button {
    display: grid;
    place-items: center;
    padding: 0 6px;
  }
</style>
