# Journal — 2026-06-04 — F3-11 implémenté

## Résumé

Implémentation du split adaptatif sur `ResponseFormat` dans `pipeline.rs`.
Plan analysé et corrigé la veille (voir `2026-06-03-plan-f3-11-analysis.md`).

## Ce qui a été fait

### Step 1 — `llm_translate_with_split` avec `Box::pin`

- Fonction privée ajoutée dans `pipeline.rs` avant `translate_batch`
- Retourne `Pin<Box<dyn Future<Output = Vec<(usize, String, bool)>> + Send + 'a>>`
- `indices: Vec<usize>` owned pour compatibilité `async move` sans lifetime conflicts
- `LlmProvider: Send + Sync` déjà super-traits → pas de bound redondant nécessaire
- `#[allow(clippy::type_complexity)]` sur la signature (type retour Pin<Box<dyn...>>)
- `eprintln!` existant remplacé par `log::warn!` partout
- `failed_unique_idx` supprimé — obsolète

### Correction bug découverte pendant l'implémentation

Le type du tokenizer s'appelait `Tokenized` (pas `TokenizeResult` comme dans le plan).
Fix trivial sur la signature.

### Spread dans `translate_batch`

```
for (local_idx, text, needs_review) in llm_results:
    global = to_translate[local_idx]   ← LOCAL → GLOBAL
    if needs_review → source_text fallback (conserve comportement existant)
    else            → text traduit
```

### Step 2 — Tests

Tests ajoutés :
- **Test A** `test_response_format_triggers_split` — 2 seg, 3 RF + 2 Ok → 5 calls, les 2 traduits
- **Test C** `test_split_partial_success` — 4 seg, s4 toujours KO → 11 calls, 3 traduits + 1 needs_review

Tests préexistants (tous encore valides) :
- `test_response_format_exhausted_falls_back_to_needs_review` — 1 seg, 3 RF → needs_review + source fallback ✓
- `test_placeholder_failure_falls_back_to_needs_review` — 1 seg, 3 bad restores → needs_review + source fallback ✓
- 5 autres tests pipeline inchangés ✓

## Résultats

```
cargo clippy -- -D warnings : 0 warning
cargo test pipeline          : 9 passed, 0 failed
```

## Fichiers modifiés

- `src-tauri/src/llm/pipeline.rs` — implémentation + 2 nouveaux tests
- `ROADMAP.md` — section "Robustesse LLM" F3-11 cochée
- `CHANGELOG.md` — entrées Added + Fixed dans [Unreleased]
- `docs/plans/f3-11-adaptive-batch-split.md` — statut → [x] Complet

## Comportement observable

**Avant :** un seul segment mal formaté dans un batch de 20 → TOUS les 20 passent en `needs_review`.

**Après :** le batch se coupe en deux récursivement jusqu'à isoler le segment problématique.
Seul le segment réellement défaillant passe en `needs_review`. Les autres sont traduits normalement.
Cas réaliste (1 segment sur 20 problématique) : ~7 appels LLM au lieu de 1, invisible pour l'utilisateur.

## Limitation connue (F4)

La barre de progression se fige pendant les sous-appels du split (architectural — `on_progress`
vit dans `run_inner` et n'est pas accessible à `translate_batch`). Documenté dans le plan.
