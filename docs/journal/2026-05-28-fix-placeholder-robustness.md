# Journal — 2026-05-28 — Fix : robustesse validation placeholder

**Phase** : Hotfix post-F3-03
**Durée estimée** : ~1h
**Statut** : ✅ Complété

---

## Ce qui a été fait

Correction du bug "placeholder validation failed after 3 attempts" causé par un prompt
trop long (injection glossaire) qui dégradait l'attention du LLM sur les tokens ⟦ph_N⟧.

4 fixes appliqués sur la branche `fix/placeholder-validation-robustness` :

- **Fix 1** — `provider.rs` : ajout d'un bloc `CRITICAL RULE` dans le system prompt de
  `translate()`, placé AVANT les termes de glossaire. Texte explicite interdisant toute
  modification des tokens ⟦ph_N⟧.

- **Fix 2** — `project.rs` : remplacement de l'injection naïve des 30 premiers termes par
  un filtrage par pertinence : termes dont `source_text` apparaît dans au moins un segment
  du batch courant (max 20), avec fallback sur les 10 termes les plus courts si aucun match.

- **Fix 3** — `pipeline.rs` : correction du segment_id reporté en cas d'échec de validation.
  Avant : toujours le premier segment du batch (bug d'index). Après : index réel du segment
  qui échoue `Tokenizer::restore`.

- **Fix 4** — `pipeline.rs` : stratégie de fallback au lieu d'erreur fatale. Après
  `MAX_RETRIES` (3) échecs consécutifs, le pipeline :
  - conserve `source_text` comme `target_text` temporaire
  - marque le segment `needs_review` (champ ajouté à `TranslationResult`)
  - continue le reste du batch sans le bloquer
  - émet `h2s://llm/placeholder-warning` avec `segment_id`

- **Frontend** — `llm.ts` : listener `h2s://llm/placeholder-warning` → toast sonner warning
  "⚠️ Segment X… : placeholder non préservé — marqué comme 'À réviser'"

- **DB** — `project.rs` : `UPDATE segments SET status = 'needs_review'` pour les segments
  fallback (au lieu de `'translated'`).

## Fichiers créés

- `docs/journal/2026-05-28-fix-placeholder-robustness.md` (ce fichier)

## Fichiers modifiés

- `src-tauri/src/llm/provider.rs` — CRITICAL RULE dans system prompt + test `test_system_prompt_contains_placeholder_instruction`
- `src-tauri/src/llm/pipeline.rs` — `needs_review` sur `TranslationResult`, fallback strategy, `PlaceholderWarningPayload`, emission dans `run`, test `test_placeholder_failure_falls_back_to_needs_review`
- `src-tauri/src/commands/project.rs` — filtrage glossaire pertinent + `status = needs_review` dans UPDATE
- `src/stores/llm.ts` — `warningUnlisten` + toast warning
- `CHANGELOG.md` — entrées ### Fixed

## Fichiers supprimés

*(aucun)*

## Dépendances ajoutées

*(aucune)*

## Décisions prises

**Fallback sur tous les segments `to_translate` après MAX_RETRIES** : quand un batch de
segments échoue la validation placeholder, on ne sait pas lesquels sont problématiques
(on break à la première erreur). Plutôt que d'isoler le segment fautif (complexe), on
marque tous les segments du sous-batch `to_translate` comme `needs_review`. Conservatif
mais safe — évite toute perte de données et reste visible pour le traducteur.

**`PipelineError::ValidationFailed` supprimé** : ce variant n'est plus nécessaire puisque
le pipeline ne remonte plus d'erreur sur les placeholders manquants. Les erreurs fatales
restantes (`Provider`, `Database`) restent propagées normalement.

**Filtrage glossaire côté `translate_segments` (global)** : le filtrage aurait pu être
fait par micro-batch dans `pipeline.rs`. Choix retenu : filtrer une seule fois sur
l'ensemble des segments de la commande. Suffisant pour réduire le bruit, sans refactoring
de l'API `TranslationContext`.

## Problèmes rencontrés

- `PostToolUse:Edit` hook a reformaté `pipeline.rs` après un edit → lecture intermédiaire
  nécessaire avant le prochain edit sur ce fichier. Comportement attendu (cargo fmt).

## Tâches ROADMAP cochées

*(aucune — hotfix, pas de feature)*

## Résultats tests

```
cargo test : 141 passed, 0 failed (dont 2 nouveaux : test_placeholder_failure_falls_back_to_needs_review, test_system_prompt_contains_placeholder_instruction)
cargo clippy -- -D warnings : 0 warnings
pnpm typecheck : 0 erreurs
```

## Prochaine session

F3 restant :
- TM fuzzy matching (Levenshtein, seuil 80 %)
- Export TM au format TMX
- Rapport QA exportable HTML

F4 (priorité absolue) :
- Wolf RPG v1/v2 — `engines/wolf/extractor.rs`

---
*Généré par Claude Code — Hoshi2Star*
