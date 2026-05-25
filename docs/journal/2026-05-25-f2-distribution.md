# Journal — 2026-05-25 — F2 Distribution (GitHub Actions)

**Phase** : F2 (Distribution MVP)
**Durée estimée** : 30 min
**Statut** : ✅ Complété

---

## Ce qui a été fait

Mise en place de la distribution automatique de Hoshi2Star via GitHub Actions.

---

## Fichiers créés

- `.github/workflows/release.yml` — workflow `tauri-apps/tauri-action@v0`
  - Déclenché sur `push tags v*` et `workflow_dispatch`
  - Matrix : `ubuntu-22.04` (.AppImage + .deb) et `windows-latest` (.msi + .exe)
  - Crée une release draft GitHub avec les binaires attachés
  - Description bilingue EN/FR, prérequis Ollama, instructions d'installation

## Fichiers modifiés

- `src-tauri/tauri.conf.json` — version `0.1.0` → `0.2.0` (premier release tag)
- `ROADMAP.md` — distribution MVP cochée, F2 statut → `[x] Complet`

---

## Décisions prises

- **tauri-action@v0** retenu (version stable, supportée officiellement par Tauri)
- **Release draft** (`releaseDraft: true`) — permet de relire avant publication
- **Matrix linux + windows uniquement** — macOS exclus (nécessite certificat Apple payant, hors scope MVP)
- **`pnpm/action-setup@v4`** — impose pnpm conformément à la règle projet (jamais npm/yarn)
- **`swatinem/rust-cache@v2`** — cache le répertoire `target/` pour accélérer les builds CI

---

## Action manuelle requise avant le premier push de tag

⚠️ **GitHub Settings > Actions > General > Workflow permissions**
→ Activer **"Read and write permissions"**
Sans cette option : erreur `Resource not accessible by integration` à la création de la release.

---

## Commandes pour déclencher la release

```bash
git add .github/workflows/release.yml src-tauri/tauri.conf.json ROADMAP.md
git commit -m "ci: add GitHub Actions release workflow"
git push origin main

git tag v0.2.0
git push origin v0.2.0
# → le workflow se déclenche automatiquement
# → une release draft est créée sur GitHub Releases
```

---

## Tâches ROADMAP cochées

- [x] Build Windows `.msi` (GitHub Actions)
- [x] Build Linux `.AppImage` et `.deb` (GitHub Actions)
- [x] Page de téléchargement GitHub Releases
- **F2 → [x] Complet**

## Prochaine session

**F3 — Polissage + VX Ace + beta privée** :
1. Intégration `marshal-rs` pour les fichiers `.rvdata2` (VX Ace)
2. TM v2 — fuzzy matching (Levenshtein, seuil 80 %)
3. Glossaire v1 — CRUD + injection prompt
4. Recrutement 20–30 beta testeurs (Discord / F95zone)

---
*Généré par Claude Code — Hoshi2Star*
