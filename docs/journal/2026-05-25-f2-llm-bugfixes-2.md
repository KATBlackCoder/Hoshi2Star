# Journal — 2026-05-25 — F2 LLM Bugfixes 2 (ResponseFormat retry + /no_think)

**Phase** : F2 (bugfixes post-distribution, suite)
**Durée estimée** : 20 min
**Statut** : ✅ Complété

---

## Contexte

Après les 3 bugs corrigés en session précédente (journal `2026-05-25-f2-llm-bugfixes.md`),
les toasts d'erreur sont maintenant visibles. Deux nouveaux cas observés en test manuel :

- `"Response format error: expected 4 lines, got 0"` — qwen3 met tout dans `<think>`, rien après
- `"Response format error: expected 4 lines, got 3"` — qwen3 retourne sans numérotation, compte incorrect

---

## Ce qui a été fait

### Fix A — Désactiver le thinking mode qwen3 (`provider.rs`)

**Cause** : qwen3:4b active le mode raisonnement par défaut. Dans certains cas extrêmes,
il intègre ses traductions à l'intérieur du bloc `<think>` sans rien écrire après.
Après le stripping du bloc (fix précédent), la réponse effective est vide → "got 0".

**Fix** : Prepend `/no_think\n` au user message. C'est le flag officiel Ollama/qwen3
pour désactiver le thinking sur un tour donné. Compatible avec tous les modèles
(ignoré silencieusement par les modèles non-qwen).

```rust
let prompt_body = format!("/no_think\n{}", numbered.join("\n"));
```

`strip_think_blocks` est conservé comme filet de sécurité.

---

### Fix B — Retry sur `ResponseFormat` dans la pipeline (`pipeline.rs`)

**Cause** : La boucle de retry existante ne couvrait que les erreurs de placeholder
(`Tokenizer::restore` échoue). Une `LlmError::ResponseFormat` (mauvais nombre de lignes)
était immédiatement propagée via `?` sans aucun retry.

**Fix** : Remplacer le `.map_err(PipelineError::from)?` par un `match` explicite
dans la boucle de retry :

```rust
let llm_out = match llm_result {
    Ok(out) => out,
    Err(LlmError::ResponseFormat(_)) if attempt + 1 < MAX_RETRIES => {
        attempt += 1;
        continue;  // retry
    }
    Err(e) => return Err(PipelineError::from(e)),  // Http/Unavailable → propagation immédiate
};
```

Le compteur `attempt` est partagé entre les deux types d'échec :
MAX_RETRIES=3 tentatives au total, quelle que soit la combinaison d'erreurs.

**MockProvider mis à jour** : le type interne passe de `Result<Vec<String>, String>`
à `Result<Vec<String>, LlmError>` pour pouvoir injecter des `ResponseFormat` en test.
Les tests existants (`Ok(vec![...])`) sont inchangés — Rust infère le type d'erreur.

---

## Fichiers créés

- `docs/journal/2026-05-25-f2-llm-bugfixes-2.md` — ce fichier

## Fichiers modifiés

- `src-tauri/src/llm/provider.rs` — `/no_think\n` dans `prompt_body`
- `src-tauri/src/llm/pipeline.rs` — retry sur `ResponseFormat`, MockProvider, +1 test

## Dépendances ajoutées

- aucune

---

## Tests

- **93 tests** (était 92) — +1 dans `llm::pipeline::tests` :
  - `test_response_format_error_triggers_retry` — vérifie que ResponseFormat déclenche un retry
- `cargo clippy -- -D warnings` : ✅ 0 warnings
- `cargo test` : ✅ 93/93

## Tâches ROADMAP cochées

- aucune (bugfixes hors scope ROADMAP)

## Prochaine session

**F3 — Polissage + VX Ace + beta privée** :
1. Intégration `marshal-rs` pour les fichiers `.rvdata2` (VX Ace)
2. TM v2 — fuzzy matching (Levenshtein, seuil 80 %)
3. Glossaire v1 — CRUD + injection prompt
4. Recrutement 20–30 beta testeurs (Discord / F95zone)

---
*Généré par Claude Code — Hoshi2Star*
