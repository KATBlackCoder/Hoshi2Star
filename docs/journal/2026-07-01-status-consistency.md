# Journal — 2026-07-01 — Phase 5 remédiation : cohérence #8 (Option 2) + alignements

**Phase** : F4
**Durée estimée** : 1h30
**Statut** : ✅ Complété

---

## Ce qui a été fait

Phase 5 du plan de remédiation issu de l'audit du 2026-07-01 — l'incohérence #8
(« traduit » avait 3 définitions SQL) résolue par l'**Option 2** (tranchée dans
l'audit) + alignements annexes.

- **Backend** : `get_source_files` / `get_project_stats` refactorées en façades
  sur des helpers testables `fetch_source_files` / `fetch_project_stats`
  (`&SqlitePool`). `translated_count` par fichier devient **strict**
  (`status='translated'`, plus `target_text != ''`) ; nouveau
  `needs_review_count` par fichier ; nouveau `reviewed_count` projet →
  invariant : les 4 statuts somment à `total_segments`.
- **Types IPC** : `SourceFile.needs_review_count` (`#[sqlx(default)]` — les
  requêtes à littéraux `0 as …` comme export.rs:447 retombent sur 0),
  `ProjectStats.reviewed_count`. Miroir TS dans `lib/types.ts`.
- **FileTree** : affiche `✓ N · ⚠ M` par fichier (aucun compteur n'existait) ;
  le bouton debug-inject n'apparaît que si TOUT est strictement `translated` —
  on n'invite plus à injecter des fallbacks LLM non revus.
- **SegmentStatsBar** (ProjectList) : rend aussi `reviewed` (bleu, `◎`) — la
  barre somme à 100 %.
- **Modèle par défaut unifié sur `gemma4:e4b`** (choix utilisateur, vérifié
  présent sur son Ollama local avec `gemma4:e2b`) : source TS unique
  `src/lib/constants.ts` (consommée par `stores/settings.ts` + `stores/llm.ts`),
  miroir Rust `DEFAULT_OLLAMA_MODEL` (provider.rs) avec commentaires croisés.
  Remplace la divergence à 3 voies (`q4_K_M` / `q8_0` / `qwen3:4b`).
- **Docs alignées** : CONTEXT.md/CLAUDE.md — convention d'erreur réelle actée
  (`Result<T, String>` aux frontières + enums `thiserror` par module ; pas de
  `H2sError` global) ; « skeleton/template state » remplacé par l'état réel.

## Vérification

- **Test rouge prouvé** : projet avec 4 segments (1 par statut, tous à target
  non vide) → `translated_count` retournait **4** au lieu de 1 (l'ancienne
  définition comptait tout target non vide). Vert après fix ;
  `test_counter_semantics_one_segment_per_status` fige les 4 compteurs + la
  somme.
- Gate : `pnpm typecheck` ✅ · `cargo clippy -- -D warnings` ✅ ·
  `cargo test` = **367 pass / 0 fail / 4 ignored** (+1).
- Vérification visuelle FileTree/ProjectList : voir section Problèmes.

## Fichiers créés

- `docs/journal/2026-07-01-status-consistency.md` — cette entrée

## Fichiers modifiés

- `src-tauri/src/commands/project.rs` — helpers + SQL strict + test compteurs
- `src-tauri/src/domain/types.rs` — 2 champs IPC
- `src-tauri/src/llm/provider.rs` — `DEFAULT_OLLAMA_MODEL = "gemma4:e4b"`
- `src/lib/types.ts`, `src/lib/constants.ts` — miroirs TS
- `src/components/editor/FileTree.tsx` — compteurs `✓ N · ⚠ M`
- `src/components/editor/ProjectList.tsx` — segment `reviewed`
- `src/stores/settings.ts`, `src/stores/llm.ts` — constantes partagées
- `docs/architecture.md` — section domain/types.rs (Option 2) + constantes
- `CONTEXT.md`, `CLAUDE.md` — convention erreurs + état réel
- `CHANGELOG.md`, `tasks/todo.md`

## Décisions prises

- **`gemma4:e4b`** comme défaut unifié — demandé par l'utilisateur (remplace ma
  recommandation initiale q8_0) ; tag vérifié sur son Ollama local avant de figer.
- **Constante TS dans `lib/constants.ts`** (pas dans settings.ts) :
  `settings.ts` importe déjà `llm.ts`, l'inverse aurait créé un cycle.
- La section RunPod de CONTEXT.md (pull `qwen3:4b-…-q8_0`) **non modifiée** :
  c'est un exemple de workflow cloud, pas le défaut de l'app — à ajuster par
  l'utilisateur s'il change aussi son modèle RunPod.

## Problèmes rencontrés

- Classificateur de sécurité Bash temporairement indisponible pendant la
  vérification visuelle → bascule temporaire des statuts en DB faite plus tard
  (voir suite de session) ; le gate et le test DB couvrent déjà la sémantique.

## Tâches ROADMAP cochées

- [ ] (aucune — remédiation audit)

## Prochaine session

- Phase 7 — 🟡 Couverture de tests front (Vitest + Testing Library) +
  intégration `src-tauri/tests/` (avant les refactos DRY Phase 6 et dispatch
  Phase 8). Les 4 fixes UI de Phase 3 + FileTree ✓/⚠ de Phase 5 ont leurs tests
  front en attente.

---
*Généré par Claude Code — Hoshi2Star*
