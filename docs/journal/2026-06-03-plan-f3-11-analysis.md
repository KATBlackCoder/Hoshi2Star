# Journal — 2026-06-03 — Analyse plan F3-11 (batch adaptatif)

## Contexte

Revue du plan `docs/plans/f3-11-adaptive-batch-split.md` avant implémentation.
5 idées analysées sur le code réel (`pipeline.rs`, `Cargo.toml`).
Mode analyse uniquement — aucun fichier applicatif touché.

---

## IDÉE 1 — Box::pin au lieu d'async_recursion

**Analyse du code :**
`Cargo.toml` relu ligne par ligne. `async-recursion` est **absent** — ni
dépendance directe, ni transitive (les dépendances listées sont tauri, serde,
sqlx, thiserror, regex, uuid, sha2, hex, reqwest, marshal-rs, log).

**Décision : APPLIQUÉE**

Le plan remplace `#[async_recursion::async_recursion]` par `Box::pin` avec
signature explicite :
```rust
fn llm_translate_with_split<'a, P>(
    indices: Vec<usize>,      // owned pour éviter les conflits de lifetime
    ...
) -> Pin<Box<dyn Future<Output = Vec<(usize, String, bool)>> + Send + 'a>>
```

`indices` passe de `&[usize]` (borrowed) à `Vec<usize>` (owned) car `Box::pin`
avec `async move` ne peut pas capturer des références avec des lifetimes
non-`'static` sans les nommer explicitement dans le type de retour. Posséder
le Vec évite cette complexité.

**Impact sur le plan :** Tâche "Ajouter async-recursion dans Cargo.toml"
supprimée. Signature corrigée. Note sur `P: LlmProvider + Sync + Send` ajoutée.

---

## IDÉE 2 — log::warn! au lieu d'eprintln!

**Analyse du code :**
`Cargo.toml` ligne 37 : `log = "0.4"` confirmé. Déjà utilisé ailleurs dans le
projet (manifest.rs). Les `eprintln!` dans `pipeline.rs` (ligne ~271) sont des
restes non uniformes — la codebase a clairement adopté `log`.

**Décision : APPLIQUÉE**

Tous les `eprintln!` dans les exemples de code du plan remplacés par
`log::warn!`. Format retenu :
```rust
log::warn!("[h2s] single-segment split failed on ResponseFormat \
             after {} attempts — needs_review (local pos {})",
            MAX_RETRIES, indices[0]);
```

Note : l'`eprintln!` existant à la ligne ~271 de `pipeline.rs` doit aussi être
migré vers `log::warn!` lors du refactor (ajouté dans les tâches Step 1).

---

## IDÉE 3 — Progress events dans les splits

**Analyse du code :**
`on_progress` est une closure capturée dans `run_inner` (ligne 128-129) :
```rust
for batch_ids in &batches {
    let batch_results = translate_batch(...).await?;
    done += batch_ids.len();
    on_progress(done, total);  // ← APRÈS translate_batch complet
}
```

`translate_batch` a la signature :
```rust
async fn translate_batch<P>(
    segments, provider, context, lang_pair, db
) -> Result<Vec<TranslationResult>, PipelineError>
```

Aucun paramètre callback. `run_inner` est conçu pour être sans `AppHandle`
(testabilité) — `run` est le wrapper Tauri. Passer `app_handle` à
`translate_batch` briserait cette séparation. Un callback `on_sub_progress`
est faisable mais représente un refactor du contrat de `translate_batch`.

**Décision : REJETÉE pour F3, documentée pour F4**

Le freeze de la barre de progression pendant les splits est acceptable pour le
MVP. Section "Limitations connues" ajoutée en tête de plan avec :
- Description précise du comportement visible (freeze, puis saut)
- Durée estimée (N×3 appels × ~2s/appel)
- Chemin de correction F4 : `on_sub_progress: Option<&dyn Fn()>`

---

## IDÉE 4 — Alignement indices unique_segs vs to_translate

**Analyse du code :**
Dans `translate_batch` actuel :
```rust
let mut to_translate: Vec<usize> = Vec::new(); // indices GLOBAUX dans unique_segs
let tokenized: Vec<_> = to_translate
    .iter()
    .map(|&i| Tokenizer::tokenize(&unique_segs[i].text, TokEngine::MvMz))
    .collect();
// tokenized[k] ↔ unique_segs[to_translate[k]]
```

Le plan appelait la fonction avec `(0..to_translate.len())` — des indices
**locaux** (0, 1, 2, ...) dans `tokenized`. Mais dans les exemples de code de
`llm_translate_with_split`, le plan écrivait :
```rust
unique_segs[indices[0]].id   // BUG : indices[0] est LOCAL, pas un index dans unique_segs
```

`unique_segs[0]` peut être un segment complètement différent de celui à
l'indice local 0 dans `tokenized`.

**Décision : APPLIQUÉE — bug corrigé**

Fix : le logging de l'ID segment est supprimé de l'intérieur de
`llm_translate_with_split` (qui n'a pas accès à `to_translate`). À la place :

1. La fonction log uniquement la **position locale** (suffisant pour le debug)
2. Le lookup de l'ID est fait dans `translate_batch`, **après** le retour,
   avec la conversion `global_unique_idx = to_translate[local_idx]`

```rust
// Dans translate_batch — après llm_results :
for (local_idx, text, needs_review) in llm_results {
    let global_unique_idx = to_translate[local_idx];  // LOCAL → GLOBAL
    if needs_review {
        log::warn!("[h2s] segment '{}' needs_review after split",
                   unique_segs[global_unique_idx].id);
    }
    ...
}
```

Commentaire `// IMPORTANT: indices are LOCAL into tokenized[] / to_translate[]`
ajouté à la signature dans le plan.

---

## IDÉE 5 — Profondeur maximale (depth: u8)

**Analyse du code :**
`DEFAULT_BATCH_SIZE = 20` (ligne 29). Avec `Box::pin`, la récursion est sur le
heap — pas de risque de stack overflow.

Calcul des niveaux :
```
Niveau 0 : 1 batch × 20 segments
Niveau 1 : 2 batches × 10 segments
Niveau 2 : 4 batches × 5 segments
Niveau 3 : 8 batches × 2-3 segments
Niveau 4 : ≤ 20 batches × 1 segment  → condition terminale
```
Max 5 niveaux. Pire cas théorique (TOUS les segments échouent seuls, irréaliste) :
153 appels LLM totaux. Cas réaliste (1-2 segments problématiques) : ~7-15 appels.

La condition `indices.len() == 1` garantit la terminaison dans TOUS les cas
— la profondeur est naturellement bornée par log₂(batch_size).

**Décision : REJETÉE**

`depth: u8` et `MAX_DEPTH` ajoutent de la complexité pour un cas pathologique
(modèle incapable de traduire un seul segment) qui produit correctement
`needs_review` sans garde. Note de bornage ajoutée dans le plan sous
"Architecture cible" pour référence future.

---

## Résumé des décisions

| Idée | Décision | Impact sur le plan |
|------|----------|--------------------|
| Box::pin au lieu d'async_recursion | ✅ APPLIQUÉE | Signature corrigée, task mise à jour |
| log::warn! au lieu d'eprintln! | ✅ APPLIQUÉE | 3 occurrences corrigées + tâche ajoutée |
| Progress events dans splits | ❌ REJETÉE (F4) | Section "Limitations connues" ajoutée |
| Alignement indices | ✅ APPLIQUÉE (bug réel) | Bug corrigé, logging déplacé dans translate_batch |
| depth: u8 | ❌ REJETÉE | Note de bornage ajoutée dans le plan |

## Fichiers modifiés cette session

- `docs/plans/f3-11-adaptive-batch-split.md` — plan mis à jour (analyse + corrections)
- `docs/journal/2026-06-03-plan-f3-11-analysis.md` — ce fichier

## Prochaine session

Implémenter Step 1 de F3-11 : extraire `llm_translate_with_split` avec
`Box::pin`, mettre à jour `translate_batch`, vérifier `LlmProvider: Send + Sync`.
