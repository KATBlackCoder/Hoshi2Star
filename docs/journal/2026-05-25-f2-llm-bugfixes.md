# Journal — 2026-05-25 — F2 LLM Bugfixes (3 bugs silencieux)

**Phase** : F2 (bugfixes post-distribution)
**Durée estimée** : 30 min
**Statut** : ✅ Complété

---

## Ce qui a été fait

Correction de 3 bugs identifiés par analyse statique du code LLM.
Les 3 bugs partageaient un symptôme commun : la traduction se lançait,
s'arrêtait après quelques secondes, sans rien traduire et sans message d'erreur.

---

## Fichiers créés

- `docs/journal/2026-05-25-f2-llm-bugfixes.md` — ce fichier

## Fichiers modifiés

- `src/stores/llm.ts` — ajout `toast.error()` dans le listener `h2s://llm/error`
- `src-tauri/src/commands/project.rs` — émission `h2s://llm/completed` quand `pairs.is_empty()`
- `src-tauri/src/llm/provider.rs` — stripping `<think>`, support `1. text`, 2 nouveaux tests

## Fichiers supprimés

- aucun

## Dépendances ajoutées

- aucune (crate `regex` déjà présent dans Cargo.toml)

---

## Décisions prises

- **Pas de struct `CompletedPayload`** : l'event `h2s://llm/completed` utilise déjà `serde_json::json!` directement dans le code existant — cohérence maintenue.
- **`std::sync::LazyLock`** pour le regex THINK_RE (stable Rust 1.80+) plutôt qu'`once_cell` — pas de dépendance supplémentaire.
- **Stripping avant parsing** (pas après) : garantit que les lignes `[N]` dans le bloc `<think>` ne polluent jamais `out[]`.

---

## Problèmes rencontrés et causes racines

### Bug 1 — Erreur LLM silencieuse (`src/stores/llm.ts`)

**Cause** : Le listener `h2s://llm/error` stockait bien `error: message` dans le
store Zustand, mais aucun composant ne lisait jamais `useLlmStore(s => s.error)`.
Le spinner s'arrêtait, rien ne s'affichait.

**Fix** : `toast.error(\`Erreur de traduction : ${event.payload.message}\`, { duration: 6000 })`
ajouté dans le listener, après le `set()`.

---

### Bug 2 — `pairs.is_empty()` sans event completed (`src-tauri/src/commands/project.rs`)

**Cause** : Quand tous les segments d'un fichier avaient déjà `target_text != ''`
et `status != 'untranslated'`, la requête SQL retournait 0 lignes.
La commande Tauri retournait `Ok(())` immédiatement sans émettre
`h2s://llm/completed`. Le frontend gardait `isTranslating: true` indéfiniment —
le bouton Traduire restait en mode spinner infini.

**Fix** :
```rust
if pairs.is_empty() {
    let _ = app.emit("h2s://llm/completed", serde_json::json!({ "count": 0 }));
    return Ok(());
}
```

---

### Bug 3 — Bloc `<think>` de qwen3 pollue le parser (`src-tauri/src/llm/provider.rs`)

**Cause** : `qwen3:4b` (modèle par défaut) active le mode "thinking" et préfixe
ses réponses avec un bloc `<think>…</think>`. La fonction `parse_numbered_response`
itérait sur **toutes** les lignes sans filtrer ce bloc. Si le contenu du bloc
contenait des patterns `[N]`, ils polluaient le tableau `out[]`. Si certains
indices `[N]` manquaient dans la vraie réponse, `ResponseFormat` était renvoyé.
Cette erreur propageait via `h2s://llm/error` → Bug 1 (non affiché).

Secondaire : le commentaire disait "Accept `1. text`" mais le code ne gérait
que `[1] text`. Si le modèle retournait `1. Hero` sans crochets, le résultat
tombait dans le fallback ligne-count, qui pouvait retourner les lignes brutes
avec le préfixe `"1."` inclus.

**Fix** :
1. `static THINK_RE: LazyLock<Regex>` + `fn strip_think_blocks(raw: &str) -> String`
   utilisant `(?s)<think>.*?</think>` (multiline, lazy).
2. Dans `parse_numbered_response` : `let stripped = strip_think_blocks(raw); let raw = stripped.trim();`
3. Ajout Pattern 2 (`1. text`) dans la boucle de parsing, en `else if` après Pattern 1.

---

## Tests

- **92 tests** (était 90) — +2 nouveaux dans `llm::provider::tests` :
  - `test_parse_think_block_stripped` — vérifie que `<think>` est ignoré
  - `test_parse_dot_format` — vérifie le format `1. text`
- `cargo clippy -- -D warnings` : ✅ 0 warnings
- `pnpm typecheck` : ✅ 0 erreurs

## Tâches ROADMAP cochées

- aucune (bugfixes hors scope ROADMAP)

## Prochaine session

**F3 — Polissage + VX Ace + beta privée** (inchangé) :
1. Intégration `marshal-rs` pour les fichiers `.rvdata2` (VX Ace)
2. TM v2 — fuzzy matching (Levenshtein, seuil 80 %)
3. Glossaire v1 — CRUD + injection prompt
4. Recrutement 20–30 beta testeurs (Discord / F95zone)

---
*Généré par Claude Code — Hoshi2Star*
