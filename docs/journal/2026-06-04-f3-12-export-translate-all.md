# Journal — 2026-06-04 — Export All + Translate All avec cooldown

**Phase** : F3
**Durée estimée** : 3h
**Statut** : ✅ Complété

---

## Ce qui a été fait

- Ajout bouton "Export All" (`Download`) dans la toolbar — vérifie la complétude avant export
- Ajout dialog bloquant si segments non traduits : compte affiché, Close uniquement (pas d'export partiel)
- Ajout dialog de confirmation si tout est traduit : nb fichiers + nb segments → bouton Exporter
- Ajout commande Rust `get_project_stats` — retourne `{ fileCount, totalSegments, untranslatedCount }` via une seule requête SQLite avec binding `?1`
- Ajout bouton "Translate All" (`Languages`) dans la toolbar — bouton distinct de "Translate" (sélection)
- Ajout `TranslateAllDialog` : affiche nb segments à traduire + nb fichiers + 2 inputs durée travail/pause (défaut 20 min / 3 min, modifiables)
- Ajout commande Rust `translate_all_segments` — tâche de fond `tokio::spawn`, fichier par fichier, cooldown automatique basé sur `Instant::elapsed()` vs threshold, émet `h2s://llm/cooling { remainingSecs }` chaque seconde pendant la pause
- Ajout `isCooling`, `cooldownRemaining`, `startTranslateAll`, `coolingUnlisten` dans `useLlmStore`
- Ajout sélecteurs `useIsCooling` + `useCooldownRemaining`
- Ajout `CooldownBadge` dans la toolbar — `Snowflake` animé + `MM:SS` en bleu pendant la pause
- Ajout clés i18n `toolbar.translateAll*` + `toolbar.exportAll*` (EN + FR)
- Enregistrement `translate_all_segments` dans `lib.rs` (`generate_handler!` + import)

## Fichiers créés

- `src/components/TranslateAllDialog.tsx`
- `docs/journal/2026-06-04-f3-12-export-translate-all.md` (ce fichier)

## Fichiers modifiés

- `src-tauri/src/commands/project.rs` — ajout `ProjectStats`, `get_project_stats`, `translate_all_segments`
- `src-tauri/src/lib.rs` — import + registration `get_project_stats`, `translate_all_segments`
- `src/stores/llm.ts` — `isCooling`, `cooldownRemaining`, `startTranslateAll`, `coolingUnlisten`, `reset` mis à jour
- `src/App.tsx` — `CooldownBadge`, bouton Translate All, state `showTranslateAll`/`translateAllStats`, handlers, prop `onTranslateAll`, `TranslateAllDialog`, import `Languages`/`Snowflake`
- `src/locales/en.json` — clés `translateAll*` + `exportAll*`
- `src/locales/fr.json` — clés `translateAll*` + `exportAll*`
- `CHANGELOG.md` — section `[Unreleased]` avec les nouvelles features
- `ROADMAP.md` — section F3-12 ajoutée

## Fichiers supprimés

- Aucun

## Dépendances ajoutées

- Aucune (icônes `Languages` et `Snowflake` déjà dans lucide-react)

## Décisions prises

- **Cooldown basé sur `Instant` côté Rust** — plus fiable que compter des segments ; le threshold est en secondes et s'accumule depuis le dernier repos, pas depuis le début total
- **Guard `threshold = max(1, cooldown_threshold_secs)`** — évite une boucle infinie si l'utilisateur passe 0
- **`untranslated_count` exclut `needs_review`** — les segments `needs_review` ont déjà un texte cible et sont exportables
- **Deux boutons séparés** (Translate + Translate All) plutôt qu'un seul bouton avec mode — UX plus explicite, pas d'ambiguïté sur la portée de l'action
- **`label` HTML natif** dans `TranslateAllDialog` plutôt que composant shadcn — évite d'installer `@/components/ui/label` pour un seul usage

## Problèmes rencontrés

- `@/components/ui/label` absent (non installé via shadcn) → remplacé par `<label>` HTML natif avec classes Tailwind
- Formatter (prettier hook) modifie les fichiers après chaque Edit → nécessite re-lecture avant éditions successives dans le même fichier

## Tâches ROADMAP cochées

- [x] F3-12 — Export All button + `get_project_stats`
- [x] F3-12 — Translate All button + `translate_all_segments` + cooldown
- [x] F3-12 — `CooldownBadge` + store cooling state

## Prochaine session

- Tester le flux complet sur un vrai projet MV/MZ (cooldown, progression, pause visuelle)
- Docs + screenshots de la toolbar mise à jour
- Préparer release v0.3.2 quand stable
- F4 : démarrer Wolf RPG (priorité absolue)

---
*Généré par Claude Code — Hoshi2Star*
