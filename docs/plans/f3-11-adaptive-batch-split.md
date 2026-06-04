# Plan F3-11 — Batch adaptatif : split récursif sur ResponseFormat

## Objectif

Quand un batch de N segments échoue après MAX_RETRIES sur `ResponseFormat`
(LLM qui saute un numéro de ligne), couper le batch en deux et réessayer chaque
moitié indépendamment. La récursion s'arrête à la taille 1 : un segment seul qui
échoue encore → `needs_review` (comportement actuel, conservé comme fallback
ultime).

**Résultat attendu :**
- Cas normal : 1 appel LLM pour 20 segments (identique à aujourd'hui)
- Cas dégradé (1 segment problématique) : split automatique, ~7 appels au
  total, 20/20 traduits sans intervention de l'utilisateur
- Le bouton "Retry failed" devient un dernier recours, pas la voie principale

## Statut : [x] Complet — 2026-06-04

## Prérequis

- `translate_batch` dans `pipeline.rs` déjà en place avec retry `ResponseFormat` — `[x]` Fait
- Fallback `needs_review` déjà câblé pour les erreurs `ResponseFormat` épuisées — `[x]` Fait
- Tests unitaires MockProvider déjà opérationnels — `[x]` Fait

## Estimation

2 steps · ~30–45 min total

## Périmètre

**Backend uniquement.** Aucun changement frontend, Tauri command, ou SQL.
Seul fichier modifié : `src-tauri/src/llm/pipeline.rs`.

---

## Limitations connues

### Progress events pendant les splits (à corriger en F4)

Le frontend reçoit `h2s://llm/progress` uniquement dans `run_inner`, **après** que
`translate_batch` retourne entièrement. La barre de progression se fige pendant
toute la durée des sous-appels LLM d'un split.

Exemple : 100 segments (5 batches × 20), le batch 3 nécessite 7 appels LLM pour
split → l'UI affiche 40/100 pendant ~14 s sans mise à jour, puis saute à 60/100.

**Cause racine** : `on_progress` est une closure capturée dans `run_inner` et
n'est pas accessible à `translate_batch` (séparation volontaire pour la
testabilité — `run_inner` n'a pas de `AppHandle`). Ajouter un callback
`on_sub_progress` à `translate_batch` est faisable mais hors périmètre MVP.

**À F4** : passer `on_sub_progress: Option<&dyn Fn()>` à `translate_batch` →
`llm_translate_with_split` pour des mises à jour granulaires.

---

## Steps

---

### Step 1 — Extraire `llm_translate_with_split` avec `Box::pin`

**Objectif :** Refactorer `translate_batch` pour isoler la logique LLM dans une
fonction récursive. Quand `ResponseFormat` épuise ses retries ET que le batch
contient plus d'un segment, couper en deux et rappeler récursivement.

**Fichier touché :**
- `src-tauri/src/llm/pipeline.rs` ← modifier

**Dépend de :** *(aucun)*

#### Choix d'implémentation : `Box::pin` (pas `async-recursion`)

`async-recursion` est **absent** de `Cargo.toml` et n'est pas une dépendance
transitive. Le projet minimise volontairement les dépendances. On utilise donc
`Box::pin` avec un type de retour explicite `Pin<Box<dyn Future<...>>>`.

Conséquence : `indices` doit être `Vec<usize>` (owned) pour éviter les conflits
de lifetime avec les références dans la closure `async move`. Les tranches
`&[usize]` nécessitent des lifetimes explicites incompatibles avec `Box::pin`
sans `'static`.

#### Architecture actuelle

```
translate_batch(segments)
  ├── dedup_by_hash
  ├── TM lookup (par segment unique)
  └── [retry loop]
      ├── provider.translate(texts_for_llm)
      ├── Tokenizer::restore (validation placeholders)
      └── sur ResponseFormat épuisé → needs_review (TOUS les segments)
```

#### Architecture cible

```
translate_batch(segments)
  ├── dedup_by_hash
  ├── TM lookup (par segment unique)
  └── llm_translate_with_split(indices=0..N, unique_segs, tokenized, provider, context)
        ├── [retry loop MAX_RETRIES]
        │   ├── provider.translate(texts_for_llm)
        │   └── Tokenizer::restore
        └── sur échec épuisé (ResponseFormat ou placeholder) :
              si len > 1 → split en [0..n/2] + [n/2..n], appel récursif
              si len == 1 → needs_review (fallback terminal)
```

#### Sémantique des indices — CRITIQUE

`indices` est un `Vec<usize>` de **LOCAL indices** dans `tokenized[]` (et par
extension dans `to_translate[]`). Ce ne sont PAS des indices dans `unique_segs`.

```
to_translate = [3, 7, 12]    ← indices globaux dans unique_segs
tokenized    = [tok3, tok7, tok12]  ← aligné avec to_translate

indices passés à llm_translate_with_split = [0, 1, 2]  ← LOCAL
  → tokenized[0] = tok3  ✓
  → tokenized[1] = tok7  ✓

Dans translate_batch, après retour :
  for (local_idx, text, nr) in results:
      let global_idx = to_translate[local_idx]  // ← conversion LOCAL → GLOBAL
```

Pour éviter ce lookup complexe à l'intérieur de `llm_translate_with_split`
(qui n'a pas `to_translate`), le logging de l'ID segment ne se fait **pas** dans
la fonction récursive — seulement une position locale.

#### Signature de la nouvelle fonction interne

```rust
use std::future::Future;
use std::pin::Pin;

/// Traduit les segments aux positions `indices` (LOCAL dans `tokenized`).
/// Retourne `Vec<(local_idx, translated_text, needs_review)>`.
///
/// Si toutes les retries échouent (ResponseFormat ou placeholder) et que le
/// slice contient plus d'un segment, se rappelle récursivement sur chaque moitié.
/// Un segment seul qui échoue encore → `(idx, String::new(), true)` (needs_review).
fn llm_translate_with_split<'a, P>(
    // IMPORTANT: indices are LOCAL into tokenized[] / to_translate[] — NOT into unique_segs[]
    indices: Vec<usize>,
    unique_segs: &'a [batch::UniqueSegment],
    tokenized: &'a [crate::llm::tokenizer::TokenizeResult],
    provider: &'a P,
    context: &'a TranslationContext,
) -> Pin<Box<dyn Future<Output = Vec<(usize, String, bool)>> + Send + 'a>>
where
    P: LlmProvider + Sync + Send,
{
    Box::pin(async move {
        // ... corps ci-dessous
    })
}
```

> `P: LlmProvider + Sync + Send` est requis parce que `&P` doit être `Send`
> pour que le `Future` retourné soit `Send` (exigé par tokio multi-thread).
> Vérifier que `LlmProvider` a déjà `Send + Sync` dans sa définition dans
> `provider.rs` — si non, ajouter les bounds sur le trait.

#### Corps de `llm_translate_with_split`

```rust
Box::pin(async move {
    // 1. Extraire les textes tokenisés pour ce slice
    // IMPORTANT: tokenized[i] où i est un index LOCAL
    let texts_for_llm: Vec<String> = indices.iter()
        .map(|&i| tokenized[i].text.clone())
        .collect();

    let mut attempt = 0u32;
    let restored_texts: Vec<String> = loop {
        let llm_result = provider
            .translate(texts_for_llm.clone(), context.clone())
            .await;

        let llm_out = match llm_result {
            Ok(out) => out,
            Err(LlmError::ResponseFormat(_)) if attempt + 1 < MAX_RETRIES => {
                attempt += 1;
                continue;
            }
            Err(LlmError::ResponseFormat(_)) => {
                // Retries ResponseFormat épuisées — split ou fallback terminal
                if indices.len() > 1 {
                    let mid = indices.len() / 2;
                    let left = indices[..mid].to_vec();
                    let right = indices[mid..].to_vec();
                    let mut results = llm_translate_with_split(
                        left, unique_segs, tokenized, provider, context,
                    ).await;
                    results.extend(
                        llm_translate_with_split(
                            right, unique_segs, tokenized, provider, context,
                        ).await,
                    );
                    return results;
                } else {
                    log::warn!(
                        "[h2s] single-segment split failed on ResponseFormat \
                         after {} attempts — needs_review (local pos {})",
                        MAX_RETRIES, indices[0]
                    );
                    return vec![(indices[0], String::new(), true)];
                }
            }
            Err(e) => {
                // Erreur réseau/non-récupérable — marquer needs_review sans tuer le batch
                log::warn!("[h2s] non-recoverable LLM error during split batch: {e}");
                return indices.iter()
                    .map(|&i| (i, String::new(), true))
                    .collect();
            }
        };

        // 2. Validate + restore placeholders
        // IMPORTANT: tokenized[local_idx] où local_idx ∈ indices
        let mut fail_at: Option<usize> = None;
        let mut restored = Vec::with_capacity(llm_out.len());
        for (pos, (resp, &local_idx)) in llm_out.iter().zip(indices.iter()).enumerate() {
            match Tokenizer::restore(resp, &tokenized[local_idx].map) {
                Ok(r) => restored.push(r),
                Err(_) => {
                    fail_at = Some(pos);
                    break;
                }
            }
        }

        if fail_at.is_none() {
            break restored;
        }

        attempt += 1;
        if attempt >= MAX_RETRIES {
            // Placeholder invalide épuisé — même split récursif
            if indices.len() > 1 {
                let mid = indices.len() / 2;
                let left = indices[..mid].to_vec();
                let right = indices[mid..].to_vec();
                let mut results = llm_translate_with_split(
                    left, unique_segs, tokenized, provider, context,
                ).await;
                results.extend(
                    llm_translate_with_split(
                        right, unique_segs, tokenized, provider, context,
                    ).await,
                );
                return results;
            } else {
                log::warn!(
                    "[h2s] single-segment split failed on placeholder \
                     after {} attempts — needs_review (local pos {})",
                    MAX_RETRIES, indices[0]
                );
                return vec![(indices[0], String::new(), true)];
            }
        }
    };

    // 3. Succès — (local_idx, text, needs_review=false)
    indices.into_iter()
        .zip(restored_texts.into_iter())
        .map(|(i, text)| (i, text, false))
        .collect()
})
```

#### Mise à jour de `translate_batch`

Remplacer le bloc `retry loop` + `if let Some(fu_idx)` par :

```rust
if !to_translate.is_empty() {
    // Tokenize (aligné avec to_translate : tokenized[k] ↔ unique_segs[to_translate[k]])
    let tokenized: Vec<_> = to_translate
        .iter()
        .map(|&i| Tokenizer::tokenize(&unique_segs[i].text, TokEngine::MvMz))
        .collect();

    // indices = 0..N (LOCAL dans tokenized / to_translate)
    let local_indices: Vec<usize> = (0..to_translate.len()).collect();

    let llm_results = llm_translate_with_split(
        local_indices,
        &unique_segs,
        &tokenized,
        provider,
        context,
    ).await;

    // Spread : local_idx → to_translate[local_idx] → unique_segs → orig_idx
    for (local_idx, text, needs_review) in llm_results {
        let global_unique_idx = to_translate[local_idx]; // LOCAL → GLOBAL
        if needs_review {
            log::warn!(
                "[h2s] segment '{}' needs_review after split",
                unique_segs[global_unique_idx].id
            );
        }
        for &orig_idx in idx_map
            .get(&unique_segs[global_unique_idx].hash)
            .into_iter()
            .flatten()
        {
            translations[orig_idx] = Some((text.clone(), false, needs_review));
        }
    }
}
```

#### Bornage de la récursion — pas de `depth: u8` nécessaire

Avec `DEFAULT_BATCH_SIZE = 20`, la profondeur max est log₂(20) ≈ 5 niveaux.
`Box::pin` alloue sur le heap → pas de stack overflow. La condition
`indices.len() == 1` garantit la terminaison dans tous les cas. Le cas
pathologique (tous les segments échouent seuls) est irréaliste avec qwen3 et
produirait correctement `needs_review` pour chaque segment individuel.

Tâches :
- [ ] **NE PAS** ajouter `async-recursion` à Cargo.toml — utiliser `Box::pin`
- [ ] Vérifier que `LlmProvider` est `Send + Sync` dans `provider.rs`
      (ajouter les super-traits si absent : `trait LlmProvider: Send + Sync`)
- [ ] Extraire `llm_translate_with_split` comme fonction privée avec `Box::pin`
- [ ] Remplacer le retry loop + `failed_unique_idx` dans `translate_batch`
      par `llm_translate_with_split` + conversion `local → global`
- [ ] Supprimer `failed_unique_idx: Option<usize>` (devenu obsolète)
- [ ] Remplacer l'`eprintln!` existant (ligne ~271 dans pipeline.rs actuel)
      par `log::warn!` lors du refactor
- [ ] Vérifier compilation sans warnings clippy

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

Commit message : `feat(llm): adaptive batch split — Box::pin recursion on ResponseFormat instead of needs_review`

---

### Step 2 — Tests

**Objectif :** Couvrir les cas clés du split adaptatif avec MockProvider.

**Fichier touché :**
- `src-tauri/src/llm/pipeline.rs` ← section `#[cfg(test)]`

**Dépend de :** Step 1

#### Tests à ajouter

**Test A — `test_response_format_triggers_split`**

Scénario : batch de 2 segments, le LLM retourne `ResponseFormat` sur le batch
complet (3 fois), puis réussit sur chaque segment seul.

```
Appel 1 : [s1, s2] → ResponseFormat (attempt 0)
Appel 2 : [s1, s2] → ResponseFormat (attempt 1)
Appel 3 : [s1, s2] → ResponseFormat (attempt 2 = MAX_RETRIES) → split
Appel 4 : [s1]     → Ok("A")
Appel 5 : [s2]     → Ok("B")
```

Assertions :
- `provider.calls() == 5`
- `results[0].translated_text == "A"`, `needs_review == false`
- `results[1].translated_text == "B"`, `needs_review == false`

**Test B — `test_single_segment_split_needs_review`**

Scénario : 1 seul segment, LLM retourne `ResponseFormat` 3 fois → ne peut pas
splitter (taille 1) → `needs_review`.

Assertions :
- `provider.calls() == 3`
- `results[0].needs_review == true`
- Le test `test_response_format_exhausted_falls_back_to_needs_review` (existant)
  couvre déjà ce cas via `run_inner` — ce test le vérifie via `translate_batch`
  directement si elle est rendue `pub(crate)` pour le test, sinon via `run_inner`

**Test C — `test_split_partial_success`**

Scénario : batch de 4 segments, seul s4 échoue en permanence.

```
Appel 1 : [s1,s2,s3,s4] → ResponseFormat (×3) → split
Appel 2 : [s1,s2]       → Ok(["A","B"])
Appel 3 : [s3,s4]       → ResponseFormat (×3) → split
Appel 4 : [s3]          → Ok("C")
Appel 5 : [s4]          → ResponseFormat (×3) → taille 1 → needs_review
```

Assertions :
- `provider.calls() == 3 + 1 + 3 + 1 + 3 = 11`
- `results[0].translated_text == "A"`, `needs_review == false`
- `results[1].translated_text == "B"`, `needs_review == false`
- `results[2].translated_text == "C"`, `needs_review == false`
- `results[3].needs_review == true`

> Note : le MockProvider reçoit les segments dans l'ordre du split. Les appels
> 1/3/5 retournent ResponseFormat, les appels 2/4 retournent Ok. L'appel 5
> épuise ses 3 retries et retourne needs_review. Prévoir exactement 11 réponses
> dans la queue du MockProvider.

Tâches :
- [ ] Ajouter Test A
- [ ] Adapter Test B (vérifier si `test_response_format_exhausted_falls_back_to_needs_review`
      existant reste valide — il devrait, car avec `indices.len() == 1` le
      comportement est identique)
- [ ] Ajouter Test C (le plus critique — couvre le split partiel)
- [ ] Vérifier que tous les tests passent

```bash
cargo test --manifest-path src-tauri/Cargo.toml pipeline
```

Résultat attendu : ≥ 10 passed, 0 failed

Commit message : `test(llm): adaptive batch split — 3 new tests covering split/fallback cases`

---

## Tests obligatoires avant push

```bash
# Rust — tous les tests pipeline
cargo test --manifest-path src-tauri/Cargo.toml pipeline
# Résultat attendu : ≥ 10 tests passed

# Rust — clippy strict
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
# Résultat attendu : 0 warning

# Rust — format
cargo fmt --manifest-path src-tauri/Cargo.toml

# TypeScript — pas de changement mais vérification de cohérence
pnpm typecheck
# Résultat attendu : 0 erreur

# Test fonctionnel (pnpm tauri dev) :
# 1. Ouvrir Items.json (~80 segments)
# 2. Lancer traduction complète
# 3. Vérifier que les segments qui échouaient (ex. ligne 18) sont maintenant
#    traduits et non marqués needs_review
# 4. Vérifier qu'un batch normal (sans erreur LLM) prend le même temps qu'avant
```

## Mise à jour après complétion

- `ROADMAP.md` : cocher F3-11 sous "Robustesse LLM"
- `CHANGELOG.md` : section `[Unreleased]` → entrée `Fixed`
- `docs/journal/` : nouvelle entrée `YYYY-MM-DD-f3-11-adaptive-batch-split.md`
