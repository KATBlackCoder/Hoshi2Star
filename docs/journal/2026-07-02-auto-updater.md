# Journal — 2026-07-02 — Feature : auto-updater in-app

**Phase** : Hors remédiation (feature) — post-Phase 8
**Durée estimée** : 3h
**Statut** : ✅ Complété (code + docs) — validation E2E réelle différée

---

## Ce qui a été fait

- Intégré `tauri-plugin-updater` v2 + `tauri-plugin-process` v2 : au démarrage,
  check GitHub Releases (`latest.json`), dialog Oui/Non, download avec barre de
  progression, redémarrage sur la nouvelle version. Cf. ADR-007.
- Backend : commande `updater_supported()` (`cfg!(windows) || env APPIMAGE`) qui
  gate l'UI aux installs auto-updatables ; plugins branchés dans `lib.rs` ;
  `capabilities/default.json` → `updater:default` + `process:allow-restart` ;
  `tauri.conf.json` → `plugins.updater { pubkey, endpoints, windows.installMode }`
  + `bundle.createUpdaterArtifacts: true`.
- Front : store Zustand `updater` (machine à états, `dismissed_version` persisté),
  `UpdateDialog.tsx` (AlertDialog available/downloading/ready), `UpdateBadge.tsx`
  (icône toolbar visible si `dismissed`), check déclenché au mount dans `App.tsx`.
  i18n EN/FR (`updater.*`).
- CI : `release.yml` reçoit les env `TAURI_SIGNING_PRIVATE_KEY`
  (+ `_PASSWORD`) sur le step `tauri-action`.
- Tests : `src/stores/updater.test.ts` — 11 cas Vitest (plugins mockés) couvrant
  available / no-update / non-supporté / dismiss-persist+reopen / rejet non
  re-proposé / version plus récente écrase le rejet / progression→ready /
  postpone→idle / échec→error / applyAndRestart→relaunch / offline silencieux.
- Gate complet vert : `pnpm typecheck` + `pnpm test` (34, dont 11 nouveaux) +
  `cargo clippy -D warnings` (0) + `cargo test` (367 unit + 3 intégration).

## Fichiers créés

- `src-tauri/src/commands/app.rs`
- `src/stores/updater.ts`
- `src/stores/updater.test.ts`
- `src/components/UpdateDialog.tsx`
- `src/components/UpdateBadge.tsx`
- `docs/adr/ADR-007.md`

## Fichiers modifiés

- `src-tauri/src/commands/mod.rs` — `pub mod app;`
- `src-tauri/src/lib.rs` — plugins updater/process + `updater_supported` au handler
- `src-tauri/Cargo.toml` — deux crates plugin
- `src-tauri/tauri.conf.json` — `plugins.updater` + `createUpdaterArtifacts`
- `src-tauri/capabilities/default.json` — `updater:default`, `process:allow-restart`
- `.github/workflows/release.yml` — env de signature
- `package.json` — `@tauri-apps/plugin-updater` + `@tauri-apps/plugin-process`
- `src/App.tsx` — check au mount + `<UpdateDialog/>`
- `src/components/AppToolbar.tsx` — `<UpdateBadge/>` dans le cluster droit
- `src/locales/{en,fr}.json` — section `updater`
- `docs/architecture.md` — store `updater`, commande `app.rs`, index ADR-006/007
- `CHANGELOG.md` — entrée Added sous [Unreleased]
- `.gitignore` — `*.key` + `.tauri/` (filet de sécurité clés)

## Dépendances ajoutées

- Rust : `tauri-plugin-updater = "2"`, `tauri-plugin-process = "2"`
- npm : `@tauri-apps/plugin-updater`, `@tauri-apps/plugin-process`

## Décisions prises

- Endpoint = release GitHub `latest.json` (pas de serveur de manifeste) → ADR-007.
- Signature minisign obligatoire ; clé privée hors dépôt (`~/.tauri/`), secret CI.
- UI gatée par `updater_supported` : deb/rpm/dev n'affichent aucune UI d'update.
- État `ready` explicite (Redémarrer / Plus tard) plutôt que relaunch auto.

## Problèmes rencontrés

- Permission `process:allow-relaunch` inexistante → la bonne est
  `process:allow-restart` (build échouait exit 101). Corrigé.

## ⚠️ Prérequis opérationnel bloquant (utilisateur)

- `createUpdaterArtifacts: true` rend la **signature obligatoire à chaque
  release**. Vérifié : `gh secret list` est **vide** — le secret
  `TAURI_SIGNING_PRIVATE_KEY` n'est PAS posé → le prochain tag `v*` **échouera
  en CI** tant qu'il ne l'est pas. Action user : `gh secret set
  TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/hoshi2star.key` (+ `_PASSWORD` si définie).
- `/releases/latest/` ignore les drafts : l'update n'est servie qu'après
  publication manuelle de la release (`releaseDraft: true`).
- Validation E2E réelle (download+install signé sur AppImage/Windows) impossible
  avant : (a) secret posé, (b) une release updater-enabled publiée, (c) la
  suivante. À tester manuellement à partir de 0.4.4 → 0.4.5.

## Prochaine session

- Poser le secret `TAURI_SIGNING_PRIVATE_KEY` avant tout `v0.4.4`.
- Remédiation restante : Phase 9 (perf) — option ROADMAP, sur signal.

---
*Généré par Claude Code — Hoshi2Star*
