# Tasks — Hoshi2Star

## Analyse complète de l'app (2026-10-25) — SESSIONS 1 À 3 FAITES, Wolf (2 bis) à faire

Session dédiée, à lancer à part. Prompt prêt : `docs/plans/2026-10-25-analyse-complete-app.md`
(méthode « deeper, not bigger » de Felix Rieseberg). Aucun code avant la fin de l'analyse.

- [x] Outillage : `.mcp.json` corrigé, MCP `tauri` cassé supprimé, Context7 ajouté,
      skills Tauri de Codex liés pour Claude (vérifié : `claude mcp list` connecté, 2026-10-25).
- [ ] Plugin officiel `code-review` : installé par l'utilisateur ? À vérifier en début de session.
- [x] Lancer l'analyse en 3 sessions (découpage dans le plan) ; livrable `docs/audit/2026-10-analyse-complete.md`.
      Sessions 1, 2 (MV/MZ) et 3 faites le 2026-10-25. Reste : session 2 bis (Wolf, après REA).
- [ ] **À corriger, demande utilisateur du 2026-10-25 (constat P-6 / S-L2) :** quand Ollama ou un autre fournisseur tombe pendant
      une traduction, afficher **une seule** notification, pas un toast par segment. La rafale a
      saturé WebKitGTK (8,7 Go), qui s'est tué lui-même. À traiter avec S-L2 : arrêter l'exécution sur échec
      du fournisseur, ne pas écrire la source comme cible. **Rattaché au lot R1** (ci-dessous). Aucun code avant son plan.

## Lots de fiabilisation R (analyse complète, session 3, 2026-10-25) — NON DÉMARRÉ

Détail, preuves et critères : `docs/audit/2026-10-analyse-complete.md`, « Lots proposés ».
Un plan détaillé par lot avant tout code. Ordre conseillé : R1, R2, R3, R4, R5, puis R6 à R10.
Les cases sportsfan30 (A–D) ci-dessous restent la liste détaillée de R4, R8, R9, R10.

- [x] **R1 — Panne du fournisseur : arrêt propre et une seule notification** (majeur, priorité 1, M) — fait, fusionné le 2026-10-25 (PR #10, `af0c08e`) ; plan ci-dessous :
      S-L2 / audit B, P-6, S-L4, demande utilisateur (une seule notification) ; ne jamais écraser
      une cible non vide sur échec technique (retraduction par sélection, `translate.rs:34-41`). `split.rs:155-161`,
      `pipeline.rs:315`, `:577-593`, `stores/llm.ts:224-230`, `translate.rs:241`.
- [ ] **R2 — Export sûr après une mise à jour du jeu** (bloquant, M–L) — **fait (R2a, E2, R2b), gate cloud vert, PR ouverte ; étape 7 reportée** : S-X1 / audit F, P-2 ; inclut
      la reconstruction du snapshot (`engines/mv_mz/originals.rs:320-345`).
- [ ] **R3 — Dépendances** (majeur, S) : rustls / webpki (S-D2), zip (S-D1), `deny.toml` (S-D4). S-D5 traité par le retrait du crate (plan RW0 ci-dessous). Modifie `Cargo.lock`.
- [ ] **R4 — Export partiel et par fichier** (S–M) : lot A sportsfan30 + S-X5 (jamais d'écriture en place).
- [ ] **R5 — Éditeur fiable** (majeur, S chacun) : S-U1, S-U2, S-U3, S-U4.
- [ ] **R6 — Qualité des cibles LLM** (majeur, M) : P-5a (détection incomplète), S-L1, reste du point A
      (`%n` des `skill_message`/`state_message`, perte de `%n` non critique), S-L3, S-L5. Absorbe 2c du 2026-07-14.
- [ ] **R7 — Projets copiés et anciens** (M) : S-C4, S-C5, audit E, langue par défaut (S-C7, P-9).
- [ ] **R8 — QA lisible et révisable** (M) : lot B sportsfan30, audit D, S-X7, S-C6.
- [ ] **R9 — Navigation recherche** (S) : lot C sportsfan30.
- [ ] **R10 — Couverture MV/MZ** (à évaluer) : commandes 320/324/325, text fitting MV (lot D sportsfan30), `plugins.js` sur jeu réel.
- [x] **RW0 — Retirer `wolfrpg-map-parser`** — fait, gate cloud vert, fusionné le 2026-10-25 (PR #11, `252bcff`).
- [~] **RW — Wolf** : plan détaillé validé le 2026-10-26 (« Plan détaillé — RW », plus bas) ; branche `feat/rw-wolf` (worktree `../Hoshi2Star-rw`).
- [ ] Plus tard : S-C1, S-C2, S-C3, S-E3, S-E4, S-X6, S-X8 ; mineurs S-L6–L8, S-U5, S-U6, S-E10, S-E12. Non fait : S-E9 (VX Ace désactivé), S-D3.

### Plan détaillé — R1 : panne du fournisseur, arrêt propre et une seule notification (2026-10-25, Claude) — IMPLÉMENTÉ sur `fix/r1-provider-outage` (étapes 1–10, essai réel compris) ; PR #10 ouverte ; gate cloud du 2026-10-25 sur `98236f1` (R1 + `main` avec RW0) : fmt OK, typecheck OK, Vitest 133 OK (35 fichiers), clippy `--locked -D warnings` OK, `cargo test --locked` 695 OK / 0 échec / 12 ignorés, lint 0 erreur (10 avertissements, les mêmes que sur `main`) ; tests R1 ciblés et tests RW0 tous verts

**Constats couverts.** S-L2 / audit B (P-6, `PROBE B`), demande utilisateur du 2026-10-25 (une seule
notification), écrasement en retraduction par sélection (`translate.rs:34-41`, relecture session 3).
S-L4 (annulation) : étape 7, **incluse dans R1** (décision utilisateur du 2026-10-25).
Preuves et lignes : `docs/audit/2026-10-analyse-complete.md` (P-6, « Seconde relecture », R1).

**Comportement actuel (vérifié dans le code de `main` @ `fb6be51`).**
- `provider.rs:447-459` : 3 essais sans délai sur toute erreur, y compris 401 ; puis `LlmError::Http`
  ou `TranslationFailed`. `LlmError::Unavailable` (`:136`) n'est jamais produit par `translate`.
- `split.rs:155-161` : toute erreur autre que `ResponseFormat` rend `(i, "", true)` pour tout le lot.
- `pipeline.rs:577-593`, `:661-666` : cible vide ⇒ source copiée en cible, `needs_review` ; la QA tourne
  sur cette copie (`unchanged_source`). Le lot est enregistré sans garde de statut (`pipeline.rs:153-163`).
- `pipeline.rs:313-326` : un événement `h2s://llm/placeholder-warning` par segment `needs_review`.
- `stores/llm.ts:224-230` et `:294-300` : un `toast.warning` par événement, sans `id` ni regroupement.
- `translate.rs:313-316` (et `:141`) : `h2s://llm/completed` même si tout a échoué ; seul
  `health_check` (`:76-83`, `:213-220`, au démarrage) émet `h2s://llm/error`.
- `translate.rs:241-245` : « Tout traduire » ne reprend que `untranslated` ou cible vide.

**Comportement visé.**
1. Panne globale du fournisseur (connexion refusée, délai dépassé, 401/403, 404 modèle, 5xx ou 429
   persistants) ⇒ l'exécution s'arrête, **un seul** message d'erreur, avec le nombre de segments déjà
   traduits et restants.
2. Échec technique, global ou limité à un lot ⇒ **aucune écriture** pour les segments concernés : cible
   et statut restent ceux d'avant. Un segment non traduit reste `untranslated` ; une traduction existante,
   même `reviewed`, n'est jamais remplacée par la source.
3. Fin d'exécution ⇒ **un seul** toast récapitulatif : traduits, à réviser (par type d'erreur QA),
   échecs techniques. Plus aucun toast par segment.
4. « Tout traduire » relancé après la panne reprend exactement les segments non traités.

**Décisions de conception — validées par l'utilisateur le 2026-10-25 (D1 à D7, D6 = oui).**
- D1. Classement des erreurs dans `provider.rs` (`translate`) :
  - global, sans relance : 401, 403, 404 ⇒ `LlmError::Unavailable` ;
  - global, après relances avec attente croissante (1 s, 2 s, 4 s ; `Retry-After` respecté, plafonné à 30 s) :
    connexion refusée, délai dépassé, 429, 5xx ⇒ `LlmError::Unavailable` ;
  - limité au lot : 400, 413, 422 (prompt trop long, requête refusée) ⇒ `TranslationFailed` ;
  - réponse illisible ⇒ `ResponseFormat` (inchangé : relances puis découpage).
  `chat()` (`:548-581`) suit déjà en partie ce modèle : aligner les deux sur une même fonction de classement.
- D2. `split.rs` : `Unavailable` remonte (le `SplitOutcome` porte une erreur d'arrêt) au lieu d'être
  absorbé ; `TranslationFailed` et l'échec final de `ResponseFormat` sur un segment deviennent des
  résultats « échec technique » distincts de `needs_review`.
- D3. `pipeline.rs` : un résultat « échec technique » n'est ni passé à la QA, ni enregistré, ni compté
  comme traduit. Sur `Unavailable`, le pipeline renvoie `Err(PipelineError::Provider…)` après avoir
  enregistré les lots déjà terminés (comportement actuel de `persist_batch_results`, conservé).
  Nouvelle variante `PipelineError::ProviderUnavailable` pour que la commande distingue panne et erreur de base.
- D4. Événements : supprimer l'émission par segment de `h2s://llm/placeholder-warning` ; le récapitulatif
  part dans la charge de `h2s://llm/completed` (`{ count, translated, needsReview, failed, qaErrorsByType }`)
  ou de `h2s://llm/error` (`{ message, translated, remaining }`). Évite aussi ~1 000 événements IPC.
  Les autres écouteurs de `llm/completed` (`SegmentGrid.tsx:277`, `GlobalSearchResults.tsx:91`)
  n'utilisent pas la charge : compatibles.
- D5. Frontend `stores/llm.ts` : retirer `onWarning` et ses toasts ; un `toast` récapitulatif à
  `completed` (succès simple si 0 à réviser et 0 échec, avertissement sinon) ; un `toast.error` unique
  à `error`, avec un `id` fixe pour qu'une erreur répétée remplace la précédente. Textes dans
  `src/locales/fr.json` et `en.json`.
- D6. **Décidé : oui.** « Tout traduire » reprend aussi les anciennes lignes `needs_review` avec cible = source
  (`status = 'needs_review' AND target_text = source_text`), laissées par les versions actuelles.
  Le comptage initial (`translate.rs:181`) et la requête par fichier (`translate.rs:241-245`) doivent
  utiliser la même condition. Test : une ligne `needs_review` cible = source est reprise ; une ligne
  `needs_review` avec une vraie cible ne l'est pas.
- D7. `health_check` : vérifier aussi que le modèle choisi figure dans `/models` quand la liste n'est pas
  vide (`provider.rs:518-519`), pour échouer au démarrage plutôt qu'au premier lot.

**Étapes (chacune testée avant la suivante).**
- [x] 1. Classement des erreurs HTTP dans `provider.rs` (D1). Tests httpmock (`provider.rs:812+`) :
      401 ⇒ 1 seul appel puis `Unavailable` ; 503 ×3 ⇒ `Unavailable` après attentes ; 503 puis 200 ⇒ succès ;
      429 avec `Retry-After: 1` respecté ; 400 ⇒ `TranslationFailed` ; port fermé ⇒ `Unavailable`.
      Les attentes doivent être injectables (durées nulles en test) pour ne pas ralentir la suite.
- [x] 2. `split.rs` : propagation de `Unavailable`, résultat « échec technique » (D2). Tests unitaires avec
      un fournisseur simulé : `Unavailable` ⇒ arrêt sans découpage ; `TranslationFailed` ⇒ échec des seuls
      indices du lot ; `ResponseFormat` inchangé (relances, découpage, échec isolé).
- [x] 3. `pipeline.rs` : pas de repli sur la source, pas de QA ni d'écriture pour un échec technique,
      `ProviderUnavailable` (D3), récapitulatif compté. Tests d'intégration (base temporaire, comme
      `PROBE B`) :
      - fournisseur toujours en panne ⇒ `Err`, 0 ligne modifiée, segments `untranslated` ;
      - panne après le 1er lot ⇒ 1er lot enregistré, le reste `untranslated` ;
      - retraduction d'un segment `reviewed` avec panne ⇒ cible et statut inchangés ;
      - `TranslationFailed` sur un lot ⇒ exécution `Ok`, `failed = taille du lot`, segments inchangés ;
      - vraie erreur QA critique ⇒ toujours `needs_review` avec la cible du modèle (non-régression).
      Inverser l'attente de la probe `b_provider_failure` (`docs/audit/…session3.probes.rs`) dans un vrai test.
- [x] 4. Commandes `translate.rs` : sur `ProviderUnavailable`, arrêt de toute la boucle de fichiers, une seule
      émission de `h2s://llm/error` avec le récapitulatif ; `refresh_stats` du manifeste appelé aussi en cas
      d'arrêt ; `completed` porte le récapitulatif (D4). Reprise D6 selon la décision. D7.
      Vérifier `pilot.rs:379`, `:401` (même pipeline) : une panne doit y donner une erreur claire.
- [x] 5. Frontend `stores/llm.ts` (D5) : tests Vitest (`src/stores/llm.test.ts`) :
      1 000 avertissements simulés ⇒ 0 toast ; `completed` avec récapitulatif ⇒ 1 toast ;
      `error` ⇒ 1 `toast.error`, `isTranslating = false` ; textes fr/en présents.
- [x] 6. Nettoyage : retirer `PlaceholderWarningPayload` (`progress.rs:15`) et les commentaires
      `pipeline.rs:13`, `:212`, `:367` s'ils ne servent plus ; mettre à jour `CONTEXT.md` (liste des
      événements `h2s://llm/*`) si elle les cite.
- [x] 7. Annulation (S-L4, incluse dans R1) : bouton « Arrêter » sans toucher à Ollama ; arrêt vérifié entre deux lots, le lot en cours se termine d'abord ; commande `cancel_translation` + jeton d'annulation vérifié
      entre deux lots ; bouton « Arrêter » ; même récapitulatif unique. Tests : annulation après le 1er lot
      ⇒ 1er lot enregistré, reste `untranslated`, `completed` ou `cancelled` émis une fois.
- [x] 8. Gate complet : `pnpm typecheck && pnpm test && cargo clippy --manifest-path src-tauri/Cargo.toml
      -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml` ; `cargo fmt`.
- [x] 9. Vérification réelle (reprise de P-6) — faite le 2026-10-25, résultats dans `docs/journal/2026-10-25-r1-panne-fournisseur.md` (« Essai réel ») ; écart corrigé ensuite : le récapitulatif compte les réparations de noms canoniques (`repaired`), sur la copie de la session 2 (`XDG_DATA_HOME=…/test/_session2/xdg-data
      pnpm tauri:linux`, jamais la base originale) : « Tout traduire » sur un projet jetable, arrêt d'Ollama
      en cours (`sudo systemctl stop ollama`, par l'utilisateur). Attendu :
      - un seul toast d'erreur, aucun toast par segment ;
      - segments restants `untranslated`, aucune cible = source nouvelle (requête SQL avant/après) ;
      - mémoire du processus WebKit stable (relevé `ps` comme `sl2-03`) ;
      - Ollama relancé, « Tout traduire » reprend exactement les segments restants et termine.
      Même essai en retraduction par sélection de segments déjà traduits : cibles inchangées.
- [x] 10. Suivi : journal, `tasks/handoff.md`, CHANGELOG (skill `update-changelog`), leçon si correction.

**Écarts et précisions de l'implémentation (2026-10-25).**
- D2 étendu : l'échec final de validation des placeholders est aussi un « échec » (rien d'écrit), pas un
  `needs_review` avec la source. Même raison : ne jamais remplacer une cible par la source.
- D6 appliqué aussi à « Traduire » sur un fichier (même condition `RESUMABLE_SEGMENT`, `translate.rs`).
  La boîte « Tout traduire » affiche le nouveau compteur `resumableCount` (`get_project_stats`, même règle) ;
  `untranslatedCount`, qui bloque l'export, est inchangé (R4).
- D7 limité au préréglage Ollama (`require_listed_model`) : le routeur Hugging Face accepte des ids
  (`model:provider`, `:fastest`) absents de sa liste `/models`, la vérification bloquerait une configuration valide.
- « Arrêter » interrompt aussi une pause de refroidissement (`CooldownState::maybe_rest`), sans test automatique
  (dépend d'un `AppHandle`).
- Délai dépassé ⇒ panne globale (D1) : un lot trop long pour le délai de 180 s arrêterait l'exécution à
  chaque relance. Visible et reprenable ; à surveiller à l'étape 9.
- Gate du 2026-10-25 : typecheck, 129 tests Vitest, clippy `-D warnings`, 705 tests Rust (12 ignorés),
  lint 0 erreur (10 avertissements déjà présents). Relancé à l'identique après la correction de la pause, `resumableCount` et D7 limité à Ollama : tout vert.

**Critère de sortie.** Les 4 points du « comportement visé » vérifiés par tests automatiques (étapes 1 à 5)
et par l'essai réel (étape 9), gate vert.

**Hors périmètre.** Qualité des relances (lot 2 du 2026-07-14), fuites de contexte (R6), filtres QA (R8),
Pas de nouveau statut de segment en base (pas de migration) : un échec
technique laisse simplement la ligne telle qu'elle était.

**Risques.**
- Les tests existants qui attendent le repli sur la source (`needs_review`, cible = source) vont échouer :
  les relire un à un, n'inverser que ceux qui décrivent le défaut.
- Un 400 dû à un prompt trop long pourrait se répéter sur tout le projet : il reste limité au lot, mais le
  récapitulatif doit le rendre visible (`failed` élevé).
- Attente croissante : ne pas bloquer l'interface ; les durées restent côté backend, la progression continue.

**Effort estimé.** M, 1,5 à 2 jours (annulation comprise).

### Plan détaillé — R2 : export sûr après une mise à jour du jeu (2026-10-25, Claude) — PLAN VALIDÉ le 2026-10-25 (E1 = a, E2 oui, E3 = a, E4 oui, E5 oui) ; R2a (étapes 1–6) implémentée le 2026-10-26 sur `fix/r2a-export-source-guard`, gate complet en attente (réseau) ; R2b en attente de validation

**Constats couverts.** S-X1 / audit F (P-2 : traduction écrite dans le nom de visage d'une commande 101,
partie MV bloquée en jeu ; MZ : export qui échoue avec `key not found in JSON`), aggravé par la relecture
de la session 3 (copie de référence qui remet du japonais sans dérive). Preuves : rapport, P-2 et
« Seconde relecture ».

**Comportement actuel (lu dans le code de `main` @ `af0c08e`).**
- Les segments MV/MZ sont adressés par pointeur JSON (`/events/6/pages/0/list/5/parameters/0`). Un patch
  du jeu qui insère une commande décale tous les pointeurs suivants de la page.
- Export : la requête lit `source_text` (`export.rs:369-372`), puis le jette (`:396-398`). `inject_to_bytes`
  n'accepte qu'un pointeur existant sur une chaîne (`injector.rs:30-40`). `verify_json_edit`
  (`integrity.rs:80-124`) compare la sortie à la copie de référence, jamais à `source_text`. Le contrôle
  équivalent existe pour `plugins.js` seulement (`OriginalChanged`, `integrity.rs:287-293`).
- Copie de référence : `ensure_fresh_mv_mz_snapshot` (`core/originals.rs:83`, appelé par l'export) relit
  le jeu. Si le fichier a changé, `reconstruct_json` (`engines/mv_mz/originals.rs:310-370`) remet la
  source sous chaque ancien pointeur dont la valeur « n'a plus de japonais » (`stale_export`, `:331-337`).
  Après une insertion, un nom de visage `Actor1` sous un ancien pointeur de dialogue devient donc une
  réplique japonaise, **sans être compté en dérive**. `update_snapshot` (`core/originals.rs:277-310`)
  adopte la reconstruction avec un simple `log::warn`.
- La dérive n'apparaît que dans la boîte de couverture (`coverage.rs:383-398`), sans bloquer quoi que ce soit.
- L'export d'un seul fichier (`debug_inject_file`, `export.rs:1052+`) suit le même chemin, et écrit en
  place (S-X5, traité par R4).

**Comportement visé.**
1. Aucune traduction n'est jamais écrite sous un pointeur dont la valeur d'origine n'est plus la source
   enregistrée. L'export refuse et liste les segments concernés (fichier, emplacement, attendu, trouvé),
   avec le dialogue d'intégrité qui existe déjà. Rien n'est écrit.
2. La reconstruction de la copie de référence ne modifie plus une valeur dont le rôle a changé (un nom de
   visage, un choix…) : elle la signale en dérive.
3. Après une mise à jour du jeu, l'utilisateur peut **réaligner** son projet : les traductions suivent
   leurs répliques à leur nouvel emplacement, les répliques nouvelles arrivent non traduites, celles qui
   ont disparu sont listées. Sans cela, le point 1 bloquerait l'export pour toujours.

**Décisions — validées par l'utilisateur le 2026-10-25 : E1 = (a), E2 oui, E3 = (a), E4 oui, E5 oui.**
- E1. Écart source / jeu à l'export :
  - (a) **recommandé** : refuser tout l'export, avec la liste. Même règle que la porte d'intégrité
    actuelle, qui ne produit jamais d'archive partielle ;
  - (b) exporter le reste en ignorant les segments en écart, avec un avertissement.
- E2. Reconstruction : la règle `stale_export` ne s'applique plus que si le **rôle** de la valeur est le
  même. Pour une commande d'événement, le `code` de la commande et l'index du paramètre doivent
  correspondre au type du segment (dialogue = 401 ou 405, `parameters/0` ; locuteur = 101,
  `parameters/4` ; choix = 102, `parameters/0/n`). Dans les autres fichiers, le chemin est inchangé par
  construction. Sinon, la valeur est laissée telle quelle et signalée en dérive. Recommandé : oui.
- E3. Réalignement (point 3) :
  - (a) **recommandé** : dans R2, en seconde partie (R2b), car sans lui une mise à jour bloque l'export ;
  - (b) dans un lot séparé, plus tard.
- E4. Règle d'appariement du réalignement (si E3 = a) : par fichier et par page d'événement. On apparie
  d'abord les répliques à source identique dans l'ordre (plus longue sous-suite commune, comme un `diff`),
  puis rien d'autre : pas d'appariement approximatif d'une source modifiée. Une source modifiée devient
  « nouvelle », non traduite, et l'ancienne traduction est proposée dans la mémoire de traduction.
  Recommandé : oui.
- E5. Vérification : le gate en cloud, plus des tests d'intégration qui reproduisent P-2 sur un jeu
  synthétique. L'essai en jeu sur la copie de la session 2 (`test/_ingame/MV_SX1`) demande de lancer l'app
  en local : il n'est fait que si tu l'acceptes.

**Étapes — R2a : refuser l'écart (M).**
- [x] 1. `integrity.rs` : nouvelle variante `IntegrityIssue::SourceChanged { file, pointer, expected, found }`,
      avec son libellé dans `summarize`. Tests unitaires.
- [x] 2. `export.rs` (`export_project`, branche JSON MV/MZ) : avant `inject_to_bytes`, comparer pour chaque
      ligne la valeur d'origine (copie de référence parsée une fois par fichier) à `source_text`. Tout écart
      donne `SourceChanged`. Les lignes de mise en page (`split_fitted`) gardent leur pointeur, donc le contrôle
      se fait avant. Les étiquettes Keke et `plugins.js` ont déjà leurs propres contrôles.
- [x] 3. Même contrôle dans `debug_inject_file` (export d'un seul fichier).
- [x] 4. Frontend : le dialogue d'intégrité (`integrityIssuesOf`, `useAppHandlers.ts`) affiche ces écarts
      lisiblement, avec fichier, emplacement, attendu et trouvé, plus une phrase « le jeu a changé depuis
      l'extraction » et le lien vers le réalignement (R2b). Textes fr/en.
- [x] 5. Tests d'intégration (fait dans `tests/export_game_update.rs`, jeu synthétique écrit par le test) reproduisant P-2 :
      - insertion d'un 401 avant une ligne traduite ⇒ export refusé, `SourceChanged` listé, aucune archive ;
      - même chose en MZ, avec un nom de locuteur et un choix (fin de `key not found in JSON`) ;
      - jeu inchangé ⇒ export identique à aujourd'hui (non-régression des allers-retours).

**Étapes — R2a bis : reconstruction sans changement de rôle (S–M).**
- [x] 6. `reconstruct_json` : règle E2 (contrôle du `code` et de l'index de paramètre à partir de
      `segment_kind`, déjà en base). Tests : nom de visage sous un ancien pointeur de dialogue ⇒ laissé tel
      quel et compté en dérive ; ancienne cible exportée au même rôle ⇒ toujours remise à la source.
  **Fait le 2026-10-26 (Claude)** — étapes 1 à 6, détail et preuves : `docs/journal/2026-10-26-r2a-garde-source-export.md`.
  - Test rouge puis vert (`tests/export_game_update.rs`) ; la garde seule (1–3) laissait passer P-2 en MV
    (copie de référence reconstruite avec le japonais sous le pointeur décalé), d'où l'étape 6 dans R2a.
  - Rôle (étape 6) déduit du pointeur et du `code` de la commande sur disque, affiné par `segment_kind` ;
    appliqué aussi à `was_exported`. Le lien vers le réalignement (étape 4) viendra avec R2b (étape 9) :
    le dialogue dit pour l'instant de remettre la version du jeu utilisée à l'extraction.
  - Vérifié avec un dictionnaire lindera factice (`DOCS_RS=1`, `lindera.dev` refusé par le réseau cloud) :
    gate complet **à refaire** avec le vrai dictionnaire. Faux positifs sur jeux réels : non vérifiés.
  - Relecture indépendante : décalage vers une commande de même code (`♪～`) corrigé par comparaison de la
    page avec la copie précédente (`same_page`) ; refus sur un ancien export non reconnu (cible CJK, mise en
    page) **conservé** (E1), message élargi — décision utilisateur attendue.
- [ ] 7. **Reportée** (décision utilisateur du 2026-10-26 : le refus de l'étape 2 suffit à empêcher l'écriture décalée). `update_snapshot` : la dérive est rendue à l'appelant (pas seulement dans le journal), pour que
      l'export puisse dire « le jeu a changé » même avant le contrôle de l'étape 2.

**Étapes — R2b : réaligner après une mise à jour (L, si E3 = a).**
- [x] 8. Commande `resync_project` : ré-extraction des fichiers changés depuis la copie de référence, avec
      la même extraction qu'à l'ouverture. Appariement E4, puis, dans une transaction :
      - déplacement des traductions (cible, statut, score, contexte) vers les nouveaux pointeurs ;
      - insertion des nouvelles répliques en `untranslated` ;
      - segments disparus conservés à part, jamais supprimés en silence (statut ou table d'orphelins :
        à préciser au moment du code) ;
      - copie de référence remplacée par la nouvelle version.
- [x] 9. Interface : action « Réaligner après une mise à jour du jeu », proposée quand l'export signale un
      écart ou que la couverture voit une dérive. Récapitulatif unique : déplacés, nouveaux, disparus,
      sources modifiées.
- [x] 10. Tests : insertion, suppression, modification d'une réplique, page réordonnée, plusieurs fichiers ;
      un export après réalignement passe et chaque traduction est à sa nouvelle place.

**Plan détaillé R2b (2026-10-26, Claude) — constat 2 de la relecture R2a : refus conservé (E1), décision utilisateur du 2026-10-26.**
Lu dans le code : `open_project` / `visit_extracted_files` / `insert_segments` (`commands/project.rs`), `scan_project` /
`enable_category` (`commands/coverage.rs`), `SnapshotStore` (`engines/mv_mz/originals.rs`), `tm::insert`.
- Source de vérité : la copie de référence **après** `ensure_fresh_mv_mz_snapshot` (elle suit déjà la nouvelle version
  du jeu, exports défaits). Rien n'est relu ailleurs ; la copie n'est pas réécrite, seule sa dérive est effacée.
- Nouvelles répliques attendues :
  - segments de base : `dispatch_extract` sur la copie, exactement comme à l'ouverture ;
  - catégories optionnelles qui ont des segments (dont Keke et `plugins.js`) : candidats du même `coverage::scan`
    que l'activation.
- Appariement (E4), fichier par fichier :
  - pointeur dans une liste de commandes (`…/list/N/parameters/…`) : groupe = la liste. Plus longue sous-suite
    commune sur `(source, type compatible)` ; `unknown` est compatible avec tout. Préfixe et suffixe communs
    retirés d'abord ; au-delà de 16 M cases, repli glouton dans l'ordre (sous-suite valide, pas forcément maximale) ;
  - autre pointeur (base de données, `displayName`, notes Keke, `plugins.js`) : même clé et même source.
  - Une source modifiée devient nouvelle et non traduite ; l'ancienne ligne part aux orphelins.
- Application, dans une transaction :
  - lignes appariées mises à jour sur place : `json_key` et contexte d'extraction ; `id`, cible, statut, score et
    catégorie conservés. L'`id` gardé préserve les occurrences terminologiques (FK en cascade) ;
  - clés passées d'abord par une valeur temporaire unique (index unique partiel des catégories) ;
  - `rowid` renuméroté dans le nouvel ordre du fichier, car `get_segments` trie par `rowid` ;
  - nouvelles lignes `untranslated` ;
  - fichier JSON nouveau dans le jeu : `source_files` + segments, puis capture de sa copie de référence ;
  - lignes disparues traduites copiées dans une nouvelle table `orphaned_segments` (migration 0012), puis supprimées ;
    les non traduites sont simplement supprimées ;
  - traductions des lignes disparues ou à source modifiée ajoutées à la mémoire de traduction (`tm::insert`).
- Après validation de la transaction : dérive effacée dans la copie de référence (`SnapshotStore::clear_drift`).
- Commande : `resync_project(project_id, apply)`. `apply = false` donne l'aperçu, sans écriture. Sinon on recalcule
  et on applique. Retour : `ResyncSummary`, avec par fichier les déplacés, nouveaux, disparus et sources modifiées,
  plus des exemples bornés. MV/MZ seulement.
- Interface : bouton « Réaligner le projet… » dans le dialogue d'intégrité quand le jeu a changé, et dans l'avis de
  dérive de la couverture. `ResyncDialog` montre l'aperçu, puis « Appliquer », puis le récapitulatif. Textes fr/en.
- Hors périmètre, à lister : réglages de mise en page par message (`text_fit_overrides`, clé = pointeur du 101) et
  catégories désactivées (`parked_segments`). Ils deviennent caducs, sans risque : chacun n'est réutilisé que si la
  source est identique. Les fichiers disparus du jeu restent exportés depuis la copie, comme aujourd'hui.

**R2b fait le 2026-10-26 (Claude)**, détail : `docs/journal/2026-10-26-r2b-realignement.md`.
- Commits : `d897f54` (appariement), `fe5f9df` (commande, migration 0012, tests), `e7cb87f` (interface).
- Tests : 9 tests d'intégration dans `export_game_update.rs` (5 pour R2b : insertion MV, locuteurs et choix MZ,
  suppression + réécriture, nouveau fichier, argument de commande de plugin) ; page réordonnée couverte en
  unitaire (`engines/mv_mz/resync.rs`).
- Gate complet réel vert le 2026-10-26 sur `608bdbe` : 717 tests Rust, 142 Vitest, clippy, fmt, typecheck, lint.
- Relecture indépendante : catégorie plus proposée laissée intacte, mémoire de traduction jamais écrasée, 4 mineurs
  corrigés ; gate réel ensuite : 719 tests Rust, 142 Vitest. Suivis listés ci-dessous.
- Limites : une catégorie optionnelle n'est pas proposée dans un fichier ajouté par la mise à jour avant une
  nouvelle activation ; `text_fit_overrides` et `parked_segments` deviennent caducs (sans risque).

**Suivis R2b (relecture du 2026-10-26, mineurs non bloquants, à planifier) :**
- [ ] Repli glouton de `pair_list` (au-delà de 16 M cases) : une source répétée lointaine peut rendre orphelines les
      lignes intermédiaires (jamais mal placées). Piste : diff de Myers en O(ND).
- [ ] Fichier sans ligne déplacée : contexte (`speaker`, `context_json`, `segment_kind` `unknown`) non rafraîchi.
- [ ] `text_fit_overrides` : clé = pointeur du 101 ; avec des messages répétés identiques, un réglage peut passer au
      mauvais message après un décalage. Piste : migrer les pointeurs avec les paires déplacées.
- [ ] `orphaned_segments` : ni lecture ni restauration dans l'interface.
- [ ] Pas de garde serveur contre un réalignement pendant une traduction (seul le bouton est désactivé).
- [ ] Récapitulatif : expliquer les limites d'E4 (page insérée avant les autres, étiquette Keke insérée, ligne que
      l'extracteur actuel ne produit plus).

**Étapes communes.**
- [x] 11. Gate complet **en session cloud** (règle du 2026-10-25) : vert le 2026-10-26 sur `608bdbe`. Essai en jeu selon E5 : **non fait** (accord de l'utilisateur requis).
- [x] 12. Suivi : journal, handoff, CHANGELOG, et fermeture de S-X1 / F dans le rapport d'audit (2026-10-26). Contrôle des faux positifs de la garde R2a sur les jeux réels de la session 2 (2026-10-26, Claude, script Python en lecture seule sur la base copiée et les copies de jeux, sans build) : **0 écart** sur 37 543 lignes de 4 projets (MV 15 498, StandGirl ancien 1 191, StandGirl neuf 1 191, 借金JK 19 663), y compris 1 294 lignes traduites ; aucune différence d'espaces ni de fins de ligne. Limite : comparaison au fichier du jeu sur disque (équivalent à la copie de référence sur ces copies jamais exportées) ; `plugins.js` et étiquettes Keke non couverts. Sortie : `test/r2-false-positives.txt` (hors Git).

**Critère de sortie.** Le scénario P-2 (401 inséré) ne peut plus produire d'archive décalée, en MV comme en
MZ. La copie de référence ne met plus de japonais dans un champ non textuel. Après réalignement, l'export
remet chaque traduction à sa nouvelle place. Gate cloud vert.

**Hors périmètre.** Wolf (RW), écriture en place de l'export d'un fichier (R4), appariement approximatif
des sources modifiées (pourra s'appuyer plus tard sur la mémoire de traduction), mise en forme JSON (S-X10).

**Risques.**
- Faux positifs du contrôle de l'étape 2 : un segment dont la source enregistrée a été normalisée à
  l'extraction (espaces, fins de ligne) ne serait plus égal. Le vérifier sur les 3 jeux de la session 2 avec
  un test d'aller-retour sans changement (0 écart attendu sur 36 352 pointeurs).
- Le réalignement touche la base sur beaucoup de lignes : une transaction unique, et un aperçu du
  récapitulatif avant d'appliquer.
- Les étiquettes Keke et les catégories optionnelles ont leurs propres clés : à couvrir dans le réalignement,
  ou à exclure explicitement et à lister.

**Effort estimé.** R2a + R2a bis : M, environ 1 jour. R2b : L, 2 à 3 jours.

### Plan — RW0 : retirer `wolfrpg-map-parser` (2026-10-25, Claude) — PLAN VALIDÉ le 2026-10-25 (W1 = a, W2–W4 oui) ; étapes 1–7 faites, gate cloud vert ; fusionné (PR #11, `252bcff`)

**Pourquoi.** Décision utilisateur du 2026-10-25 : la méthode (fork d'un crate tiers, branche Git
`fix/wolf-v3-format`) est incorrecte ; une meilleure solution sera cherchée avec REA (lot RW). Le fork
`KATBlackCoder/wolfrpg-map-parser` n'existe plus sur GitHub : un clone neuf (cloud, Codex, autre poste)
ne compile plus Hoshi2Star ; seul le cache Cargo local (`21f4f76`) permet encore la compilation ici.
Bloque aussi la vérification en cloud de R1 (PR #10).

**Ce qui dépend du crate (lu dans le code, `main`).**
- Extraction : `engines/wolf/extractor.rs` — `extract_map_segments` (branche non-LZ4, `Map::parse`, l.509-581),
  `normalize_wolf_command_signatures` (l.26), `extract_common_events` (branche non-LZ4, l.832-899).
- Injection : `engines/wolf/injector.rs` — `inject_map` (branche non-LZ4, `Map::parse` + `patch_mps_strings`,
  l.122-176, l.178-243), `inject_common_events` (branche non-LZ4 + `patch_common_events_strings`, l.560-652).
- Tests : ceux qui fabriquent ou relisent des `.mps`/`CommonEvent.dat` v2 (`make_v2_common_event*`,
  `test_v2_common_event*`, `test_real_honoka_*`, relecture `Map::parse` l.992, l.1217).
- `Cargo.toml:44` (`wolfrpg-map-parser = "0.6"`) et `:61-62` (`[patch.crates-io]` vers le fork).
- **Non touché** : `v3_format/` (cartes et `CommonEvent.dat` v3.x compressés LZ4), `dat_parser.rs`
  (bases `.dat`, maison), `decrypt/`, détection.

**Conséquence pour l'utilisateur.** Toute carte `.mps` ou tout `CommonEvent.dat` **non compressé LZ4 v3**
n'est plus lisible : Wolf v2 (Honoka) en entier, et peut-être des v3.x non compressés (non vérifié).
Aujourd'hui, une carte illisible est **ignorée en silence** (`extract_all_wolf`, `eprintln!("warn: skipping")`) :
le projet s'ouvrirait avec les seules bases `.dat`, sans prévenir. À éviter.

**Décisions à valider.**
- W1. Jeu Wolf dont des cartes ou `CommonEvent.dat` ne sont pas au format v3 LZ4 :
  - (a) **recommandé** : refus d'ouvrir le projet, avec un message clair (« Wolf RPG v2 : cartes et
    événements communs pas encore pris en charge ») ;
  - (b) ouverture avec les bases `.dat` seulement, et un avertissement visible listant les fichiers ignorés.
- W2. Projet Wolf v2 déjà en base : l'export refuse avec le même message clair (pas d'écriture partielle).
- W3. Branche dédiée `chore/remove-wolfrpg-map-parser` depuis `main`, PR à part, fusionnée **avant** R1 :
  aucune branche ne compile en cloud tant qu'elle n'est pas fusionnée. Puis R1 rebasé et vérifié en cloud.
- W4. Vérification **en cloud uniquement** (règle du 2026-10-25 : le gate fait geler le poste).

**Étapes.**
- [x] 1. `extractor.rs` : erreur dédiée `ExtractorError::UnsupportedWolfFormat` pour les branches non-LZ4 ;
      `extract_all_wolf` remonte ces erreurs au lieu de les ignorer (W1).
- [x] 2. `injector.rs` : même erreur pour `inject_map` / `inject_common_events` non-LZ4 (W2) ; supprimer
      `patch_mps_strings`, `patch_common_events_strings`, `normalize_wolf_command_signatures` et les helpers
      devenus inutiles (garder `splice_wolf_strings`/`write_wolf_string` s'ils servent encore à `serialize_dat`).
- [x] 3. Message dans l'interface : erreur d'ouverture (`openProjectErrorKey`) et d'export, textes fr/en.
- [x] 4. Tests : retirer ceux qui reposent sur le crate ; ajouter « carte v2 ⇒ erreur explicite » (extraction,
      injection) et « projet v2 ⇒ ouverture refusée » ; les tests v3 et `.dat` restent inchangés.
- [x] 5. `Cargo.toml` : retirer la dépendance et le `[patch.crates-io]` ; `Cargo.lock` mis à jour (en cloud).
- [x] 6. Docs : CHANGELOG (« Removed »), `docs/references/wolfrpg-format-compatibility.md`,
      `v3_format/mod.rs` (commentaire l.7-11), règle de `tasks/lessons.md` du 2026-06-11 (v2 n'utilise plus
      le crate), rapport d'audit (S-D5 résolu par le retrait), lots R3/RW ci-dessus.
- [x] 7. Gate complet **en session cloud** sur la branche — gate cloud du 2026-10-25 sur `32d36ad` : fmt OK, typecheck OK, Vitest 124 OK (35 fichiers), clippy `--locked -D warnings` OK, `cargo test --locked` 681 OK / 0 échec / 12 ignorés, lint 0 erreur (10 avertissements préexistants), `Cargo.lock` cohérent ; il ne reste que des commentaires mentionnant `wolfrpg`. Puis PR ; fusion par l'utilisateur.

**Fait le 2026-10-25 (non compilé localement, règle « gate en cloud »).**
- `ExtractorError::UnsupportedWolfFormat` / `InjectorError::UnsupportedWolfFormat`, marqueur `WOLF_V2_UNSUPPORTED`
  (`wolf_v2_unsupported`) ; `extract_all_wolf` et `extract_wolf_project` refusent le projet entier (3 fichiers
  cités au plus) au lieu d'ignorer les cartes en silence ; ouverture annulée par la transaction (aucun projet partiel).
- Retirés : `normalize_wolf_command_signatures`, `patch_mps_strings`, `patch_common_events_strings`,
  `splice_wolf_strings`, tests v2 du crate ; `Cargo.toml` et `Cargo.lock` édités à la main (dépendances
  `encoding_rs`/`lz4_flex` déjà directes).
- Interface : `openProjectErrorKey` ⇒ `projectList.wolfV2Unsupported` ; export ⇒ `toasts.wolfV2Unsupported` ;
  « Supportés : … Wolf RPG v3 ». Nouveaux tests : extracteur (refus, liste), injecteur (carte, événements
  communs, export), e2e `wolf_v2_game_is_refused_at_open`, Vitest `projectError.test.ts`.

**Critère de sortie.** `rg wolfrpg` ne trouve plus que l'historique (CHANGELOG, journaux) ; un clone neuf
compile en cloud ; un jeu Wolf v2 donne un message clair, jamais un projet partiel silencieux ; tests v3 verts.

**Effort estimé.** S à M (environ une demi-journée), vérification en cloud comprise.

## Études REA d'applications tierces (2026-10-25)

Dossier local `RAE-PLAN/` (ignoré par Git) : règles, index et modèle dans `RAE-PLAN/README.md`.
Une étude = une session dédiée, une question précise, un rapport. Rien de copié dans Hoshi2Star ;
les idées retenues deviennent un plan ici.

- [ ] RuneTranslate 1.3.1 — architecture multi-moteurs (17 moteurs). Plan et prompt :
      `RAE-PLAN/runetranslate/PLAN.md`. Licence à vérifier avant analyse.

## Retours utilisateur sportsfan30 (F95zone, 2026-10-25) — NON DÉMARRÉ

Contexte : jeu MV, Hoshi2Star v0.4.10, travail par petits lots testés en jeu.
Constats vérifiés dans le code de `main` (`de76e64`) le 2026-10-25. Chaque lot aura son
plan détaillé avant implémentation. **Session 3 (2026-10-25)** : A, C, D confirmés ; B confirmé en
partie (le score QA est bien calculé en continu, ce sont les filtres qui gênent). Rattachements :
lot A → R4, lot B → R8, lot C → R9, lot D → R10 (voir « Lots de fiabilisation R »).

**Lot A — Export partiel (priorité 1, bloque son flux de travail)**
- [ ] Autoriser l'export d'un projet partiellement traduit, avec avertissement : le backend
      ignore déjà les segments vides (`commands/export.rs` filtres `target_text != ''`) ;
      le blocage est dans l'UI (`exportAllBlockedDesc`).
- [ ] Export d'un ou plusieurs fichiers choisis (aujourd'hui `debug_inject_file` exige un
      fichier complet, `export.rs:1077`).
- [ ] Statut « fichier ignoré » (ex. `MapInfos.json`) : exclu des stats, du QA et de l'export.

**Lot B — QA lisible (priorité 2)**
- [ ] Score QA calculé et enregistré en continu (aujourd'hui seulement à l'audit d'export,
      `core/report.rs:285`) : la colonne QA de la grille affiche « — » sinon.
- [ ] Filtres : « Score < 100 » clairement nommé, et par type d'erreur (dont `line_too_long`).
- [ ] Cohérence compteur de projet (« X/Y ok ») / panneau du segment.

**Lot C — Navigation recherche (priorité 3)**
- [ ] Au clic sur un résultat global, faire défiler la grille jusqu'au segment
      (`GlobalSearchResults.tsx:160` sélectionne sans défiler).
- [ ] Emplacement lisible : nom/ID de l'événement + numéro de ligne au lieu du pointeur JSON brut.
- [ ] Recherche dans le fichier : garder le vrai numéro de ligne quand on filtre.

**Lot D — Text fitting pour MV (à évaluer)**
- [ ] Étudier l'extension du text fitting (MZ seulement aujourd'hui) aux jeux MV.

**Question ouverte à l'utilisateur** : modèle utilisé, Ollama ou autre fournisseur.

---

## Installation BMAD pour Codex et Claude — 2026-09-21

Périmètre : module principal BMM, intégrations locales Codex et Claude Code,
français, conservation des skills et du suivi partagé. Aucun code applicatif.

- [x] Vérifier les prérequis et la version stable officielle : BMAD 6.12.0.
- [x] Vérifier les options de l'installateur et les deux intégrations disponibles.
- [x] Installer BMM pour les deux outils dans ce dépôt.
- [x] Vérifier les fichiers installés et la préservation de l'existant.
- [x] Actualiser le point de reprise et consigner les commandes d'utilisation.

Preuves et limites : `docs/journal/2026-09-21-installation-bmad.md`.
Guide : `docs/bmad.md`. Découverte dans une nouvelle session de chaque outil
encore à vérifier ; aucun workflow applicatif n'a été lancé.

---

## 📋 PLAN — Recommandations issues des analyses concurrents (2026-07-14) — NON DÉMARRÉ

> Synthèse actionnable des analyses SLR Translator v3.022 (2026-07-14) et
> DazedMTLTool (2026-07-12). **Aucun code écrit** — chaque lot devra passer par
> son propre plan détaillé avant implémentation. ⚠️ SLR Translator est **GPL** :
> on réimplémente les *mécanismes* en Rust, on ne copie jamais de code (c'est du
> JS de toute façon). Mémoires : `slr-translator-analysis`,
> `project-dazedmtltool-analysis`.
>
> **Déjà couvert chez nous — ne PAS refaire** : filtrage du glossaire par
> occurrence (`core/glossary.rs::relevant_terms`, Phase 6), dédup de batch
> (`llm/batch.rs`), validation placeholders + split adaptatif (`pipeline.rs`),
> tokenizer structurel (ADR-002, supérieur à leurs regex d'escaping).

### Lot 1 — Quick wins pipeline LLM (petit, sûr, indépendant du reste)

- [ ] **1a. Budget de contexte unifié** (SLR `_fitContextBlocks`) : dans le
      constructeur de prompt, traiter glossaire / contexte projet / (futur)
      contexte de scène comme des blocs `{id, text, priority}` partageant un
      budget = `context_length modèle − max_tokens réponse − system − user`,
      rognés du moins prioritaire au plus prioritaire. Nécessite une estimation
      de tokens grossière (chars/3 pour JA suffit). Rentable immédiatement avec
      les petits modèles Ollama (gemma).
- [ ] **1b. ETA dans la progression** : `ProgressPayload` (progress.rs) n'a que
      `done/total`. Ajouter une estimation basée sur les **caractères** (pas les
      segments, tailles trop hétérogènes), en **excluant les hits TM** du calcul
      de débit — avantage sur SLR : eux détectent les cache-hits par heuristique
      (>1000 chars/s), nous on SAIT quels segments viennent de la TM
      (`from_tm`). Afficher `~Xmin restant` dans la toolbar/grille.
- [ ] **1c. Anti-boucle de répétition** : sur réponse dégénérée (répétition du
      même motif / finish reason `length`), bump `repeat_penalty` de +0.05 par
      retry au lieu de re-essayer à l'identique (SLR 2.084). S'ajoute au split
      adaptatif existant, ne le remplace pas.

### Lot 2 — Retry « intelligent » (enrichit pipeline.rs, iso-architecture)

- [ ] **2a. Prompt de correction détaillé** : aujourd'hui un échec de
      restauration placeholder → retry aveugle du même prompt. À la place,
      renvoyer au modèle la conversation + un message listant les problèmes PAR
      segment (« segment 3 : ⟦ph_2⟧ manquant ; segment 5 : japonais résiduel »)
      — converge beaucoup plus vite qu'un re-essai sec (SLR 2.082-2.084).
- [ ] **2b. Décision de retry basée sur l'amélioration** : comparer le nombre de
      problèmes de la réponse N à celui de N−1 ; si ça ne s'améliore pas malgré
      des instructions précises → splitter tout de suite (au lieu d'épuiser
      MAX_RETRIES). Si ça s'améliore → autoriser un essai bonus.
- [ ] **2c. Check QA « box shifting »** : nouveau check dans le QA engine —
      détecter la signature « segment A trop court + segment B adjacent trop
      long » dans un même batch (texte qui a débordé d'une boîte dans l'autre).
      Ratio lignes/caractères source↔cible. Critique pour Wolf (`@N\n` en tête).

### Lot 3 — Scene Graph (grosse feature, plan dédié obligatoire)

- [ ] **3. Contexte narratif par scène** — le vrai différenciateur relevé chez
      SLR (3.009→3.022) : reconstruire les scènes depuis les events
      (MV/MZ : codes 101/401 dialogue+locuteur, 102 choix, 117 common events →
      graphe appelant/appelé, 201 transferts ; Wolf : équivalents dans nos
      parsers de commandes). Trois consommateurs :
      - batching par scène (grandes scènes isolées entières, petites regroupées
        avec plancher de lignes, séparation dialogue / texte UI-système) ;
      - bloc `[Scene: … / Locuteur]` injecté dans le prompt (via budget 1a) ;
      - attribution du locuteur par segment (recoupe le « speaker scoring »
        Dazed — le Scene Graph le couvre et le dépasse).
      Rust, couche Engine. À planifier séparément (`/plan`) quand décidé.

### Lot 4 — À rattacher à la tâche ROADMAP F4 « providers OpenAI-compat » (en pause)

- [ ] **4a. Rate limiting adaptatif** basé sur les headers `x-ratelimit-*`
      (Dazed, pattern `AdaptiveLimiter`) — sans objet pour Ollama local, requis
      dès qu'un provider payant arrive.
- [ ] **4b. Estimation de coût pré-run** avec confirmation avant un pipeline
      complet (Dazed) — même condition.
- [ ] **4c. Formats reasoning** : `reasoning_effort` (OpenAI), `thinking` avec
      budget, `/no_think` Qwen + `chat_template_kwargs` (SLR) — à intégrer au
      design `ProviderConfig` déjà écrit dans ROADMAP F4.

### Lot 5 — Plus tard / opportuniste

- [ ] **5a. Glossaire auto-généré par LLM** (SLR 2.104→3.016) : extraction par
      chunks avec lignes « noms propres » en tête (runs katakana, kanji courts
      entre 「」), accumulation incrémentale, pause/reprise. On a déjà une
      extraction glossaire — comparer avant de décider si c'est un upgrade ou
      un doublon. Synergie avec le partage `.h2s`.
- [ ] **5b. Résumé de projet auto** persisté et injecté comme bloc de contexte
      (priorité la plus basse du budget 1a).
- [ ] **5c. Wizard de workflow guidé** (Dazed) : stepper ouvrir → traduire →
      QA → exporter avec aide contextuelle — pattern UI pur, compatible Tenmon.
- [ ] **5d. Genre des placeholders noms** (SLR 2.055) : accepter « She » pour un
      `\n[1]` déclaré féminin dans la validation — niche, qualité EN.

### Ordre recommandé

1. **Lot 1** (1a+1b ensemble : ~1 session, gate vert facile) ;
2. **Lot 2** (2a → 2b → 2c, chacun testable unitairement sur pipeline.rs/qa.rs) ;
3. **Lot 3** sur décision explicite (plan dédié) ;
4. **Lot 4** uniquement quand F4 provider expansion sort de pause ;
5. **Lot 5** au fil de l'eau.

---

## ✅ Feature — Pack d'échange `.h2s` (partage/reprise de projet) (2026-07-07) — TERMINÉE (code + vérif live), commit/release en attente

> Objectif : permettre à un traducteur de partager l'état complet de sa
> traduction (segments + statuts + glossaire ± TM) pour qu'un autre
> utilisateur de Hoshi2Star **reprenne là où il s'est arrêté**, sans jamais
> distribuer les fichiers du jeu. C'est le « format .h2s » prévu de longue
> date dans la couche Export (CLAUDE.md). Le `.hoshi2star.json` n'est qu'un
> marqueur informatif — aucune traduction dedans.

### Format du fichier

`<Jeu>.h2s` = archive ZIP contenant exactement :

| Entrée | Statut | Contenu |
|---|---|---|
| `manifest.json` | obligatoire | `formatVersion` (int, =1), `appVersion`, `engine`, `gameTitle`, `langPair`, `createdAt`, `senderProjectId`, par fichier : `{fileName, fileType, segmentCount, translatedCount}` |
| `segments.json` | obligatoire | tableau `{fileName, jsonKey, sourceText, targetText, status, qaScore}` — **uniquement les segments à cible non vide** (élimine le risque « écraser avec du vide », réduit la taille) |
| `glossary.json` | optionnel | termes projet + termes globaux utilisés, tous réimportés comme termes **projet** chez le destinataire (ne pas polluer son glossaire global) |
| `tm.tmx` | optionnel | opt-in à l'export (checkbox) ; import via l'upsert `ON CONFLICT` existant (migration 0005) |

Identité d'un segment entre deux machines : **`(fileName, jsonKey, sourceText)`** —
jamais les IDs (UUID locaux à chaque base). Vérifié sur la base dev réelle
(3 projets, >24k segments) : `(source_file_id, json_key)` unique partout,
mais AUCUNE contrainte UNIQUE ne le garantit → le matching doit rester
déterministe en présence de doublons (appariement ordinal : n-ième
occurrence ↔ n-ième occurrence).

### Validations à l'import (dans cet ordre, messages i18n dédiés)

1. ZIP lisible + `manifest.json` présent → sinon « Ce fichier n'est pas un
   pack Hoshi2Star » (cas fréquent : l'utilisateur choisit le ZIP
   `export_project` destiné aux joueurs).
2. `formatVersion` ≤ version supportée → sinon « pack créé avec une version
   plus récente de Hoshi2Star, mettez à jour ».
3. `engine` ≠ moteur du projet actif → **refus sec** (pack Wolf sur projet MV/MZ).
4. `langPair` ≠ → refus sec.
5. `gameTitle` ≠ → avertissement non bloquant (dossiers renommés, versions).
6. Anti-zip-bomb : lire uniquement les 3 entrées connues, en mémoire, taille
   décompressée plafonnée (256 Mo), jamais d'extraction sur disque avec les
   chemins de l'archive.
7. Aucun projet ouvert → « Ouvrez d'abord votre copie du jeu » (l'import
   n'extrait rien : les segments locaux doivent déjà exister).

### Dry-run OBLIGATOIRE (préviz avant toute écriture)

Chaque segment du pack est classé :
- ✅ **applicable** — `(fileName, jsonKey)` trouvé, `sourceText` identique, politique OK
- ⚠️ **conflit local** — cible locale non vide et différente → selon politique choisie
- ⚠️ **source modifiée** — clé trouvée mais `sourceText` différent (jeu mis à
  jour) → jamais appliqué par défaut ; option « appliquer en needs_review »
- ❌ **orphelin** — fichier ou clé introuvable → compté, listé dans le rapport

### Politique de collision (choix utilisateur, défaut = le plus sûr)

1. **Compléter** (défaut) : n'écrit que si le segment local est `untranslated`/cible vide
2. **Écraser sauf révisés** : remplace `translated`/`needs_review`, préserve `reviewed`
3. **Tout écraser** : confirmation forte requise

Statuts importés conservés (c'est le cœur de la « reprise ») ; `qaScore`
importé à titre indicatif, la QA live recalcule de toute façon.

### Sécurité / robustesse

- Application dans **une seule transaction SQLite** : tout ou rien.
- **Backup automatique avant import** : l'app exporte d'abord l'état courant
  dans `backup-avant-import-<date>.h2s` (app data dir) → annulation = réimport
  de ce backup en mode « Tout écraser ». (Pas d'undo en base sinon.)
- serde sur structures typées, champs inconnus ignorés (compat versions futures).
- Textes du pack = non fiables : échappés dans l'UI du rapport, repassent par
  le pipeline tokenizer/QA normal.
- Import idempotent : réimporter 2× le même pack en « Compléter » → 0 écrit.
- Pack tronqué (transfert interrompu) → CRC ZIP + parse strict → erreur propre.
- Réimport dans le projet d'origine = légitime (backup/restauration).

### Glossaire

Dédup par `(sourceText, targetText)` ; conflit (même source, cible
différente) → conserver l'existant + le signaler dans le rapport.

### UX

- Export : bouton « Partager le projet (.h2s) » barre projet → dialog save → toast.
- Import : wizard 3 étapes — fichier → rapport dry-run + choix de politique →
  rapport final « X appliqués / Y conflits / Z source modifiée / W orphelins ».
- Événement `h2s://project/import-done` ; i18n FR/EN complet.

### Implémentation

- [x] `core/h2s_pack.rs` — types serde + writer/reader ZIP (anti-zip-bomb 256 Mo,
      entrées connues seules, jamais d'extraction disque) + 8 tests round-trip
- [x] `commands/pack.rs` (module dédié plutôt qu'export.rs/project.rs) :
      `export_h2s_pack` (+ option TM), `preview_h2s_import` (dry-run typé
      `ImportBlocker`), `apply_h2s_import` (transaction unique + backup auto
      `<app_data>/backups/backup-avant-import-<date>.h2s` + événement
      `h2s://project/import-done`) — enregistrés dans le `generate_handler!`
- [x] UI : `PackExportDialog` (checkbox TM), `PackImportWizard` (rapport dry-run +
      Select politique + confirmation forte « tout écraser » + cases source
      modifiée/glossaire/TM + rapport final), 2 boutons toolbar (Share2 /
      PackageOpen), listener refresh SegmentGrid sur import-done, i18n FR/EN
      complet (`pack.*`), types TS (`ImportPreview`/`ImportReport`/…)
- [x] Tests (8 + 8) : round-trip, champs inconnus ignorés, ZIP invalide/sans
      manifest, formatVersion futur, doublons de clé (appariement ordinal),
      3 politiques, source modifiée, idempotence « Compléter », glossaire
      dédup/conflits, TM jamais écrasée
- [x] Tests Vitest wizard (3) : dry-run → apply fill → rapport ; « tout écraser »
      bloqué sans confirmation (stubs jsdom hasPointerCapture/scrollIntoView pour
      Radix Select) ; blocker moteur → message dédié
- [x] Gate complet : typecheck ✅ · **55 Vitest (+3)** ✅ · lint 0 err (8 warn
      inchangés) ✅ · clippy 0 ✅ · **418 Rust (+16) + 3 intégration** ✅
- [x] Vérif live MCP (projet Wolf réel Inko, 1948 seg) : export réel (ZIP inspecté)
      → wipe cible → preview 1 applicable/1947 identiques → apply fill → segment
      restauré + backup écrit → idempotence (0 appliqué) → fill préserve un conflit
      local + grille rafraîchie sur import-done → overwrite_except_reviewed restaure
      → refus engineMismatch (pack Wolf sur projet MV) et notAPack (package.json) ;
      dialog export vérifié à l'écran
- [x] CHANGELOG.md (Added) + journal `docs/journal/2026-07-07-h2s-exchange-pack.md`
- [x] Commit/push/release : `71a6111` (feat) + `8b92dbe` (fix ui badge) +
      `f5e301b` (docs) + `0a42ff0` (bump), tag `v0.4.10`, CI verte,
      **release v0.4.10 publiée** (latest.json → 0.4.10) le 2026-07-07

---

## ✅ Fix — Wolf : ordre @N/taille de police (2026-07-05) — TERMINÉE

> 3e bug Wolf de la journée, suite directe LZ4 (v0.4.7) + encodage (v0.4.8).
> L'utilisateur signale (captures avant/après) que le nom + portrait du
> personnage disparaissent dans certains dialogues, remplacés par le texte
> littéral « @2 » affiché. Cause : `apply_font_prefix` collait toujours
> `\f[N]` en position 0, poussant le marqueur de personnage Wolf `@N\n` hors
> de tête de chaîne — le moteur ne le reconnaît plus. Journal :
> `docs/journal/2026-07-05-wolf-font-prefix-speaker-order.md`.

- [x] `export.rs::RE_WOLF_SPEAKER_PREFIX` + `split_wolf_speaker_prefix` —
      sépare un `@N\n` en tête avant d'insérer/détecter le code de police
- [x] `apply_font_prefix` (wolf) : marqueur + code + reste, jamais code + marqueur
- [x] `scan_font_status`/`strip_font_prefix` mis à jour pour rester cohérents
- [x] Gate complet : clippy 0 ✅ · **402 Rust (+8)** ✅ (8 nouveaux tests)
- [x] Vérif live end-to-end (MCP Tauri, réapplication police 23 sur le
      projet réel) : `CommonEvent.dat` frais → `@2\n\f[23]"Haa..."` correct
- [x] CHANGELOG.md (Unreleased → Fixed) + journal
- [ ] Commit/push/release : confirmé par l'utilisateur, en cours

## ✅ Fix — Wolf v3.x unpacked : version guess + encoding mismatch (2026-07-05) — TERMINÉE

> Suite directe du fix LZ4 (v0.4.7) : l'utilisateur a restauré une copie de secours
> de `DataBase.dat`/`CDataBase.dat`, puis un nouvel export échouait avec
> « dat parse error: encoding error: invalid utf-8 sequence of 1 bytes from index 14 ».
> Cause : `guess_wolf_version_from_structure` ne sait détecter la version que via une
> archive `.wolf` — un jeu distribué sans archive (Data/ en clair, cas de Densyanai_Inko)
> retombait sur le défaut v2.0/SJIS même si ses `.dat` sont réellement v3.5/UTF-8 ;
> `serialize_dat_type` utilisait ce défaut erroné au lieu du `is_utf8` par fichier déjà
> calculé (mais ignoré), Shift-JIS-encodant tout caractère non-ASCII traduit dans une
> base UTF-8. Journal : `docs/journal/2026-07-05-wolf-unpacked-version-encoding.md`.

- [x] `detector.rs::guess_wolf_version_from_loose_dat` — fallback sondant le magic de
      `BasicData/SysDatabase.dat` avant le défaut v2.0 codé en dur (3 tests)
- [x] `injector.rs::encode_for_wolf(text, is_utf8: bool)` (au lieu de `&WolfVersion`) ;
      `serialize_dat_type` encode depuis `dat.is_utf8` (par fichier) au lieu du `version`
      externe ; `inject_dat` garde `_version` (renommé, inutilisé) pour la symétrie de
      signature avec `inject_map`/`inject_common_events` (1 test)
- [x] `dat_parser.rs::read_wolf_string` — repli Shift-JIS quand le décodage UTF-8 échoue
      sur une base déclarée UTF-8 : répare à la lecture les fichiers déjà corrompus par ce
      bug (le prochain export réussi les réécrit proprement en UTF-8) (1 test)
- [x] Gate complet : typecheck ✅ · 52 Vitest ✅ · clippy 0 ✅ · **394 Rust (+6)** ✅,
      dont `test_real_inko_database_segments` qui échouait sur le fichier réel de
      l'utilisateur avant ce fix et passe maintenant
- [x] Vérif live end-to-end (MCP Tauri, projet réel) : « Tout exporter » sans erreur,
      zip généré, re-décodage du `DataBase.dat` frais → `使用時文章[戦闘]` de アイスエッジ
      = `"cast Ice．Edge!"` (point plein cadratin correctement UTF-8, mojibake disparu)
- [x] CHANGELOG.md (Unreleased → Fixed) + journal
- [ ] Commit/push/release : en attente de confirmation utilisateur

## ✅ Feature — Recherche globale projet (concordance search) (2026-07-04) — TERMINÉE

> Plan complet : `~/.claude/plans/steady-prancing-bear.md` (approuvé).
> Barre de recherche dans le panneau FICHIERS → cherche dans TOUS les fichiers du
> projet (Entrée, min 2 chars, scope both/source/target). Résultats au centre groupés
> par fichier (headers), sélection cross-fichiers + « Traduire N lignes » (réutilise
> `translate_segments` ids explicites — retraduit inconditionnellement, badge statut
> visible), clic ligne → navigation fichier+segment.
> HORS SCOPE : FTS, édition dans les résultats, scroll-to-segment (follow-up),
> pagination au-delà du cap 500.

### Volet A — Backend Rust
- [x] A1 `domain/types.rs` : `SegmentSearchHit` (Segment + file_name) + `SegmentSearchResult`
- [x] A2 NOUVEAU `commands/search.rs` : `SearchScope` + façade `search_segments` +
      helper testable `search_project_segments` (LIKE ESCAPE, cap 500, ORDER BY
      file_name/rowid)
- [x] A3 `commands/mod.rs` + `generate_handler!` (lib.rs)
- [x] A4 Tests (6) : scope, wildcards échappés, cap+total, ordre groupé, scoping
      projet, query courte

### Volet B — Types + Store front
- [x] B1 `lib/types.ts` : SearchScope, SegmentSearchHit, SegmentSearchResult
- [x] B2 NOUVEAU `stores/search.ts` : runSearch (garde seq) / rerun / deactivate / clear

### Volet C — UI
- [x] C1 NOUVEAU `ProjectSearchBar.tsx` (draft local, Entrée, X, Select scope ;
      reset par remount `key={activeProjectId}` — pas de set-state-in-effect)
- [x] C2 `App.tsx` : barre sous FileTreeHeader + centre `searchActive ? résultats : grille`
- [x] C3 `columns.tsx` : exporter `StatusBadge`
- [x] C4 NOUVEAU `GlobalSearchResults.tsx` (rows union header/hit, virtualizer,
      sélection locale, toolbar traduire, listeners llm) + C4b thunk
      `refreshProjectData` (project.ts)
- [x] C5 i18n en+fr `projectSearch.*`

### Volet D — Tests front
- [x] D1 `stores/search.test.ts` (5) · D2 `ProjectSearchBar.test.tsx` (3) ·
      D3 `GlobalSearchResults.test.tsx` (5)

### Volet E — Vérification & clôture
- [x] Gate complet vert : typecheck ✅ · lint 0 err (8 warn inchangés + 1 classe
      useVirtualizer assumée) ✅ · 47 Vitest ✅ · clippy 0 ✅ · 387+3 Rust ✅
- [x] Vérif live MCP (projet réel MV) : « 先生 » → 500/758 en 50 fichiers, headers
      groupés, sélection Items.json+Map002.json → « Traduire 2 lignes » → 2 segments
      retraduits même lot (DB), résultats auto-rafraîchis, clic → grille du bon
      fichier + segment surligné, X → retour grille. Originaux restaurés.
- [x] CHANGELOG.md (Added) + journal `docs/journal/2026-07-04-global-project-search.md`

### Complément validé user (2026-07-04) — chargement complet au lieu du cap 500 ✅
- [x] `search.rs` : param `offset` (`SEARCH_BATCH_SIZE`, plus un cap) +
      test contiguïté inter-lots (`test_offset_batches_are_contiguous`)
- [x] `search.ts` : boucle multi-lots dans `runSearch` (pattern loadSegments,
      garde seq existante), résultats affichés progressivement
- [x] `GlobalSearchResults.tsx` : message « X / Y affichés » → indicateur
      Loader2 discret (résultats < total) ; reset sélection sur status loading
      (plus sur [results] — les lots incrémentaux ne wipent plus)
- [x] i18n : `truncated` → `loadingProgress` (en+fr)
- [x] Tests D1/D3 ajustés (mock paginé) — gate vert : 388+3 Rust · 48 Vitest ·
      clippy 0 · lint inchangé
- [x] Vérif live MCP : « 先生 » → « 758 résultats dans 78 fichiers » (vs 500/50
      tronqué avant), indicateur disparu, « Tout sélectionner » → « Traduire
      758 lignes » (décoché sans lancer)

### Complément 2 validé user (2026-07-04) — case « tout le fichier » sur les headers ✅
- [x] Checkbox tri-état sur chaque ligne d'en-tête de fichier (colonne w-9 alignée
      avec les cases des lignes) : coche/décoche tous les hits du fichier,
      indeterminate si sélection partielle ; `hitIdsByFile` (Map) mémoïsée
- [x] i18n `projectSearch.selectFile` (en+fr, avec {{fileName}} dans le title)
- [x] Test Vitest tri-état (header → 2 ids du fichier ; 1 hit → indeterminate ;
      re-clic header → complète ; dé-clic → vide) + indices tests existants ajustés
- [x] Gate vert : 49 Vitest (+1) · 388+3 Rust · clippy 0 · lint inchangé
- [x] Vérif live MCP : header Map002.json → « Traduire 12 lignes » ; 1 hit décoché
      → header indeterminate + « Traduire 11 lignes »

### Complément 3 validé user (2026-07-04) — UX vue résultats (4 modifs) ✅
- [x] Plier/déplier par fichier : chevron à droite du header, état `collapsed`
      local (reset sur nouvelle recherche), rows filtre les hits repliés ;
      sélection indépendante de l'affichage
- [x] Header sticky : barre superposée en haut du scroll (dérivée de la 1re ligne
      visible du virtualizer), masquée quand un vrai header est en 1re position ;
      remplacement instantané (pas d'effet poussé) ; contrôles embarqués
      (checkbox tri-état + chevron) — composant FileHeader partagé
- [x] Mode sélection : ≥1 coché → clic ligne = toggle sélection (pas navigation) ;
      0 coché → clic = navigation (comportement actuel)
- [x] Checkbox shadcn (Radix) dans la vue résultats uniquement (toolbar, headers,
      lignes) — `src/components/ui/checkbox.tsx` ajouté via CLI ; tri-état via
      `checked="indeterminate"` ; SegmentGrid = follow-up
- [x] i18n : `collapseFile`/`expandFile` (en+fr) ; tests D3 ajustés
      (aria-checked="mixed", nouveaux tests collapse ×2 + mode sélection ;
      sticky = vérif live, pas jsdom)
- [x] Gate complet vert (2026-07-04) : typecheck ✅ · 52 Vitest (+3) ✅ ·
      clippy 0 ✅ · 388 Rust + 3 intégration ✅ · lint 0 err / 8 warn (inchangé)
- [x] Vérif live MCP (projet réel MV, « 先生 » → 758 résultats / 78 fichiers) :
      plier Map002.json → hits masqués, compteur (12) conservé, déplier OK ;
      scroll mi-fichier → sticky « Map013.json (16) » épinglé, masqué quand un
      vrai header revient en 1re position ; 1 coché → clic ligne = toggle
      (« Traduire 2 lignes », pas de navigation) ; 0 coché → clic → grille
      Map002.json segment 8 surligné + TM exact match

### Follow-up loggé (hors scope)
- [ ] Scroll-to-segment au chargement de SegmentGrid : après navigation depuis les
      résultats, la ligne active est surlignée mais peut être hors viewport
      (scrollToIndex n'existe que dans handleTabNext)
- [ ] Harmoniser les checkboxes de SegmentGrid (columns.tsx) sur le composant
      shadcn Checkbox utilisé par la vue résultats (Complément 3)

## ✅ Tokenizer — name box `\n<Name>` : nom traduisible, structure protégée (2026-07-04) — TERMINÉE

> Problème : le Groupe G (`\\n<[^>]+>`) tokenise le code ENTIER → le nom (ハルカ) est
> opaque pour le LLM et reste en japonais. Solution validée : tokenisation scindée en
> 2 passes — passe 1 dédiée `RE_NAMEBOX = \\n<([^>]+)>` qui émet `⟦ph_i⟧Nom⟦ph_i+1⟧`
> (ph_i = `\n<`, ph_i+1 = `>`), passe 2 = regex moteur (Groupe G retiré) qui tokenise
> le reste, y compris les codes imbriqués dans la name box (`\n<\C[6]ハルカ>`).
> Garde-fou nouveau risque (LLM inverse/perd les tokens de paire) : contrôle structurel
> post-restore — nb de matchs `RE_NAMEBOX` dans le texte restauré ≥ nb d'ouvrants `\n<`
> de la map, sinon `TokenizerError::BrokenNameBox` → retry pipeline existant.
> Impacts vérifiés : qa.rs (vérifie `\n<`+`>` au lieu du nom JP verbatim — cohérent),
> filter.rs::needs_translation (segment name-box pur devient traduisible — voulu),
> pipeline/split (signatures inchangées).

### Implémentation (src-tauri/src/llm/tokenizer.rs)
- [x] `RE_NAMEBOX` static + passe 1 dans `tokenize` (MvMz + MzOnly, pas Wolf),
      counter/map partagés avec la passe 2
- [x] Retirer Groupe G de `RE_MVMZ` et `RE_MZONLY` (mort après passe 1)
- [x] `TokenizerError::BrokenNameBox` + contrôle structurel dans `restore`
- [x] Tests 15/16/17 réécrits (comportement scindé) + nouveaux : codes imbriqués,
      tokens de paire inversés → rejet, name box multiples, MzOnly, Wolf exclu,
      name-box pur → `needs_translation == true` (filter.rs)

### Vérification
- [x] Gate complet : `pnpm typecheck && pnpm test && cargo clippy -- -D warnings
      && cargo test` — vert (typecheck ✅ · 34 Vitest ✅ · clippy 0 ✅ ·
      381 Rust + 3 intégration ✅)
- [x] CHANGELOG.md (Fixed) + journal (`docs/journal/2026-07-04-tokenizer-namebox-split.md`)

## ✅ Backlog trio : TM doublons · QA live glossaire · ESLint 10 (2026-07-04)

> Branche `fix/backlog-tm-qa-lint` · Plan : `~/.claude/plans/fancy-drifting-cocke.md`
> 3 commits : `ead6731` (tm) · `7684831` (qa) · `a98ff8b` (lint).

### FIX 1 — Index UNIQUE TM + upsert
- [x] Migration `0005_tm_unique.sql` (dédup MAX(rowid) puis CREATE UNIQUE INDEX)
- [x] `tm::insert` → `ON CONFLICT(source_hash, lang_pair) DO UPDATE` + doc comment
- [x] 4 tests (double insert → 1 ligne ; id préservé ; fuzzy sans doublon ; lang_pair scopé)
- [x] Commit `fix(tm): enforce unique (source_hash, lang_pair) — dedup migration + upsert insert`

### FIX 3 — QA live : glossaire + engine
- [x] `qa_check_segment` async + State + `project_id: Option<String>` ; helper `check_segment_live`
- [x] Engine : param > DB (`projects.engine`) > `"mv_mz"` ; glossaire via `relevant_terms` (ja-en)
- [x] QAPanel.tsx : `projectId` dans invoke + queryKey
- [x] 4 tests commande (mismatch −15 ; terme respecté ; sans project_id ; engine threadé)
- [x] Commit `fix(qa): thread glossary terms and project engine into live segment check`

### FIX 2 — ESLint 10 flat config
- [x] `pnpm add -D typescript-eslint @eslint/js globals` (aucun conflit peer-deps)
- [x] `eslint.config.js` (flat, ignores dist/src-tauri/target/src/components/ui)
- [x] Script `"lint": "eslint src"` + triage : 3 fixes mécaniques, 2 règles → warn
- [x] Commit `chore(lint): migrate to ESLint 10 flat config`

### Clôture
- [x] Gate complet : typecheck + 34 Vitest + clippy -D warnings + 375/3 cargo test
- [ ] Vérif manuelle FIX 1 : lancer l'app sur la DB dev (migration 0005), requête doublons → 0 ligne
- [x] CHANGELOG.md + journal

### Follow-ups loggés (hors scope, découverts au triage lint)
- [ ] Refactor `react-hooks/set-state-in-effect` (4 warn) : effets sync-on-open dans
      SettingsModal (×2), AppToolbar, GlossaryPanel → dériver l'état ou key-reset
- [ ] Invalidation du cache `["qa-check", ...]` quand le glossaire est modifié
      (actuellement : staleTime 300 ms + navigation segment suffisent)
- [ ] `react-refresh/only-export-components` (3 warn columns.tsx) : séparer cellules
      et builders de colonnes si on veut un HMR fin

---

> **Remédiation audit 2026-07-01 — avancement**
> Ordre : 1 → 2 → 3 → 4 → 5 → 7 → 6 → 8 → 9 (`docs/audit-remediation-plan-2026-07-01.md`).
> - Phase 1 : ✅ **terminée & mergée sur `main`** (2026-07-01).
> - Phase 2 : ✅ **terminée & mergée sur `main`** (2026-07-01).
> - Phase 3 : ✅ **terminée & mergée sur `main`** (2026-07-01).
> - Phase 4 : ✅ **terminée & mergée sur `main`** (2026-07-01).
> - Phase 5 : ✅ **terminée & mergée sur `main`** (2026-07-02).
> - Phase 7 : ✅ **terminée** (2026-07-02) — 23 tests front (Vitest) + 2 tests
>   d'intégration Rust ; `pnpm test` ajouté au gate. Branche `feat/test-coverage`.
> - Phase 6 : ✅ **terminée & mergée sur `main`** (`de0f61a`, 2026-07-02).
> - Phase 8 : ✅ **terminée & mergée sur `main`** (2026-07-02) — ADR-006.
> - Remédiation : **Phase 9 (perf)** restante — option dans ROADMAP.md, sur signal.
> - 👉 **EN COURS (hors remédiation) : Feature — Auto-updater** (voir section ci-dessous).

## ✅ Feature — Auto-updater (tauri-plugin-updater) — TERMINÉE (code)

> **2026-07-02** : code + tests + docs complets sur `feat/auto-updater`, gate vert
> (typecheck + 34 Vitest + clippy 0 + 367/3 cargo), mergé sur `main`. ADR-007.
> ⚠️ **Bloquant CI non résolu (action user)** : `gh secret list` est **vide** →
> poser `TAURI_SIGNING_PRIVATE_KEY` avant tout `v0.4.4`, sinon la release échoue.

> Objectif : au démarrage, vérifier s'il existe une nouvelle version publiée ; si oui,
> proposer (Oui/Non). **Non** → badge dans le toolbar pour rouvrir le choix ; **Oui** →
> barre de progression du téléchargement puis redémarrage sur la nouvelle version.
> Doc : https://tauri.app/plugin/updater/ · Branche : `feat/auto-updater`.
>
> **Fait feature (pas bugfix)** → pas de « test rouge ». Filet = tests Vitest de la
> machine à états du store `updater` (les appels plugin `check`/`downloadAndInstall`
> sont **mockés**), dans l'esprit Phase 7. Gate complet vert avant merge.
>
> **2 réalités actées (cf. analyse)** :
> - **deb/rpm NON updatables** : l'updater ne met à jour que l'AppImage (Linux) et
>   NSIS `.exe`/`.msi` (Windows). → l'UI d'update est **gatée** : masquée si l'install
>   n'est pas auto-updatable (Linux hors AppImage). Détection : `cfg!(windows)` OU
>   `std::env::var("APPIMAGE").is_ok()`.
> - **Ne profite qu'aux versions futures** : la 0.4.3 (déjà publiée) n'a pas l'updater ;
>   la 1re build updater-enabled (ex. 0.4.4) s'installe manuellement, l'auto démarre
>   ensuite (0.4.4→0.4.5…).

### Volet 0 — Prérequis (⚠️ actions UTILISATEUR — bloquantes pour la CI)
- [x] **Générer la paire de clés** : `tauri signer generate -w ~/.tauri/hoshi2star.key`
      (à lancer côté user via `! …` ; NE PAS committer la clé privée).
- [ ] ⚠️ **NON FAIT — Ajouter les secrets GitHub** : `TAURI_SIGNING_PRIVATE_KEY`
      (+ `_PASSWORD` si définie) via `gh secret set`. **`gh secret list` est vide
      au 2026-07-02** → le prochain `v*` échouera tant que ce n'est pas posé.
- [x] Récupérer la **pubkey** générée → collée dans `tauri.conf.json` (Volet A).
- [x] **Décision de design** : endpoint = release GitHub `latest.json`
      (`https://github.com/KATBlackCoder/Hoshi2Star/releases/latest/download/latest.json`),
      confirmé. installMode Windows = `passive` (barre seule).

### Volet A — Config + backend Rust
- [x] `Cargo.toml` : `tauri-plugin-updater = "2"` + `tauri-plugin-process = "2"`.
- [x] `package.json` : `@tauri-apps/plugin-updater` + `@tauri-apps/plugin-process`.
- [x] `lib.rs` (~l.33-35) : `.plugin(tauri_plugin_updater::Builder::new().build())` +
      `.plugin(tauri_plugin_process::init())`.
- [x] `tauri.conf.json` : ajouter clé `plugins.updater { pubkey, endpoints, windows:{
      installMode:"passive"} }` + `bundle.createUpdaterArtifacts: true`.
- [x] `capabilities/default.json` : `updater:default` + `process:allow-restart`.
- [x] Command `updater_supported() -> bool` (gate deb/rpm) : `cfg!(windows) ||
      env::var("APPIMAGE").is_ok()`. Enregistrée dans le `generate_handler!` unique.

### Volet B — CI (release.yml)
- [x] Ajouter au step `tauri-apps/tauri-action@v0` les env `TAURI_SIGNING_PRIVATE_KEY`
      + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (depuis les secrets). `createUpdaterArtifacts`
      étant dans la conf, l'action générera `.sig` + `latest.json` uploadés sur la release.
- [ ] (Maintenance liée, hors updater : bumper les actions Node20→récent — logué backlog.)

### Volet C — Frontend
- [x] `src/stores/updater.ts` — slice Zustand, machine à états :
      `status: idle|checking|available|dismissed|downloading|ready|error`,
      `update: {version,notes}|null`, `downloaded`/`contentLength` (→ %),
      `dismissedVersion` (persisté via tauri-plugin-store, comme settings). Actions :
      `checkForUpdate()` (guard `updater_supported`, silencieux si offline/dev),
      `startDownload()` (callbacks `Started`→contentLength / `Progress`→+chunkLength /
      `Finished`→ready), `dismiss()` (persiste version), `reopen()`, `applyAndRestart()`
      (`relaunch()`).
- [x] `src/components/UpdateDialog.tsx` — `AlertDialog` : état `available` (notes +
      Oui/Non), état `downloading` (`Progress` shadcn + %), état `ready` (« Redémarrer
      pour appliquer »). Réutilise le pattern AppDialogs.
- [x] Badge toolbar `AppToolbar.tsx` — icône (ex. `Download`/`ArrowUpCircle`) visible
      **uniquement** si `status === "dismissed"` (ou update dispo non traitée) ; onClick →
      `reopen()`. Placement **à droite**, à côté des boutons About/Settings (`~l.280-287`).
- [x] `App.tsx` / `useAppHandlers.ts` : `checkForUpdate()` async après mount, **non
      bloquant**, silencieux sur échec. Rendre `<UpdateDialog/>` via AppDialogs.

### Volet D — Tests (Vitest, plugin mocké)
- [x] `src/stores/updater.test.ts` : `check` trouve une version → `available` ; `dismiss`
      → `dismissed` + `dismissedVersion` persistée ; `reopen` → `available` ; progression
      `Started/Progress/Finished` → % correct puis `ready` ; `check` sans update → `idle` ;
      `updater_supported=false` → aucune vérif (UI gatée) ; échec réseau → `error` silencieux.
- [x] (Pas de test Rust : `updater_supported` est trivial ; couvert par relecture.)

### Vérification
- [x] Gate complet : `pnpm typecheck && pnpm test && cargo clippy -- -D warnings &&
      cargo test` — vert.
- [x] `pnpm tauri dev` : le check ne spamme pas d'erreur en dev (guard/catch).
- [ ] **Validation réelle end-to-end** différée : ne peut être prouvée qu'après (a) clés+
      secrets en place, (b) une release updater-enabled publiée, (c) une release suivante
      pour déclencher l'update. À tester manuellement sur AppImage + Windows à ce moment-là.

### Docs
- [x] ADR (updater : endpoint GitHub Releases, signature obligatoire, deb/rpm non couverts).
- [x] `docs/architecture.md` (nouveau store + commande) + CHANGELOG (Added) + journal.
- [x] README (EN/FR) : mention « mises à jour automatiques » dans Features.

## ✅ Phase 8 — Centraliser le dispatch moteur (OCP) — TERMINÉE

> Plan §Phase 8. **Sites vérifiés sur le code réel (2026-07-02)**. Refactor **majeur**,
> **iso-comportement** : le filet est la suite Phase 7 (367 Rust + 2 intégration + 23 front) ;
> chaque étape doit laisser le gate 100 % vert **sans modifier un test existant** (ajout de
> tests de caractérisation AUTORISÉ, cf. étape 0). Toute divergence de comportement = STOP +
> re-plan. Branche : `refactor/engine-dispatch-phase-8`.
>
> **Décision de design (tranchée) : enum + méthodes, PAS de trait-objects.** Le titre « OCP »
> tenterait une réécriture en `trait Engine`, mais (a) l'ensemble des moteurs est *fermé* et
> petit (3→4 avec Bakin), (b) l'audit lui-même dit « enum `FileType` » + « ajouter Bakin = 1
> arm ». Un trait-object = diff plus gros et plus risqué pour zéro gain d'extensibilité réel
> ici. → On donne des **méthodes à `Engine`** (`db_str`, `data_dir`, `game_title`) pour tuer
> les 3 `match` de métadonnées, et on centralise la **classification des `file_type`** pour
> qu'`export.rs` cesse ses `starts_with("wolf_")`/`("vx_")` ad hoc. Trait = alternative
> **rejetée** (documentée).
>
> **Découpage en 2 commits (comme Phase 6)** — gate vert entre les deux :
> - **Commit A — décomposition pure** de la god-fn `open_project`, **sans nouvelle
>   abstraction** : extraire les helpers d'insertion + un `extract_project` par moteur
>   renvoyant *exactement* les formes actuelles.
> - **Commit B — dispatch centralisé** : introduire les méthodes d'`Engine` + la
>   classification `file_type`, router les 4 commandes.

### Sites de dispatch relevés (source de vérité)
- `project.rs::open_project` (33-284, god-fn) : `match engine` ×4 — `engine_str` (80-84),
  `data_dir` (88-94), `game_title` (97-101), extraction+INSERT (128-249, 3 bras au boilerplate
  `source_files`+`segments` dupliqué).
- `project.rs::debug_dump_segments` (~575-680) : 3-bras parallèle qui **re-détecte + ré-extrait**
  (duplique l'extraction d'open_project) ; `kind` string spécifique moteur.
- `export.rs::export_project` (229, 258, 272) : branche `has_wolf` → `collect_wolf_zip_entries`
  sinon boucle générique + sous-dispatch `starts_with("vx_")`.
- `export.rs::debug_inject_file` (498-541) : dispatch 3 voies `wolf_`/`vx_`/json.
- `export.rs` helpers font (37-51) : `engine=="wolf"` → laissés (config, pas dispatch fichier).

### Étape 0 — Filet de caractérisation (AVANT tout refactor)
- [x] Test snapshot de `debug_dump_segments` sur la fixture MV réelle (JSON figé) →
      convertit ce chemin d'« inspection seule » en « prouvé par le gate ». Ethos Phase 7.
- [x] (VX Ace `open_project` reste non couvert par e2e — moteur désactivé, pas de fixture ;
      cf. Vérification. On NE crée pas de fixture VX ici.)

### Commit A — Décomposer `open_project` (aucune abstraction nouvelle)
- [x] Extraire `insert_source_file(&mut tx, …)` + `insert_segment(&mut tx, …)` pour tuer le
      boilerplate INSERT répété 3×. **Contrainte iso critique** : les helpers prennent
      `&mut Transaction` (PAS `&pool`) — ne PAS sortir les inserts de la transaction unique
      ni changer l'atomicité (le happy-path e2e ne détecterait pas un rollback cassé).
- [x] Extraire `extract_project(engine, game_dir, data_dir) -> Vec<(file_name, file_path,
      file_type, Vec<CommonSeg>)>` — forme commune consommée par `open_project` ET
      `debug_dump_segments`. **Contraintes iso** :
      - Wolf : `file_path` est *construit* (`Data/{MapData|BasicData}/name`) — cette logique
        migre dans le bras Wolf du normalizer, ne pas la perdre.
      - Champ source : `.source` (MV/VX) vs `.source_text` (Wolf) → mapper les deux.
      - `debug_dump` : le `kind` est spécifique moteur (Wolf `"map_message"`… ; MvMz
        `format!("{:?}")`) — l'extracteur unifié doit **reproduire chacun**, ne pas collapser.
      - VX : garder l'appel `vx_extractor::extract_from_bytes` intact, juste re-câbler sa
        sortie (chemin non couvert e2e → relecture).
- [x] Gate complet vert, aucun test existant modifié (test étape 0 inclus).

### Commit B — Dispatch centralisé (enum + méthodes)
- [x] `Engine::db_str(&self) -> &'static str` → remplace le `match` de `engine_str` (80-84) et
      la construction manuelle dans manifest/insert.
- [x] `Engine::data_dir(&self, game_dir) -> Result<PathBuf,String>` → remplace (88-94).
- [x] `Engine::game_title(&self, game_dir, data_dir) -> Option<String>` → remplace (97-101).
- [x] ~~`Engine::from_db_str(&str) -> Option<Engine>`~~ **RETIRÉ avant merge** : le
      consommateur export.rs prévu ne s'est pas matérialisé (font scopé hors périmètre) →
      code mort masqué au `clippy dead_code` par `pub`. Retiré par discipline de portée
      (relevé advisor).
- [x] Classification `file_type` centralisée (ex. `FileClass::{Wolf,VxAce,Json}` +
      `fn classify(file_type: &str) -> FileClass`) → `export.rs` (229/272/498) et
      `has_wolf`/font-prefix consomment ça au lieu de `starts_with`.
- [x] Router `open_project`, `debug_dump_segments`, `export_project`, `debug_inject_file` via
      ces méthodes/classification.
- [x] Gate complet vert, aucun test existant modifié.

### Vérification (⚠ trous de couverture explicites — advisor)
- [x] `pnpm typecheck && pnpm test && cargo clippy -- -D warnings && cargo test` — vert,
      aucun test existant modifié.
- [x] **Couvert par e2e** : `open_project`+`export_project` pour **MV et Wolf** (fixtures
      réelles). Le round-trip Wolf CE verrouille toujours Phase 1.
- [x] **NON couvert par e2e → prouvé autrement** :
      - `debug_dump_segments` → snapshot étape 0 (gate-proven).
      - `debug_inject_file` → relecture (pas de test e2e) ; vérifier les 3 voies inchangées.
      - `open_project` **bras VX Ace** → relecture seule (moteur désactivé, sans fixture) ;
        appel extracteur intact.
- [x] Diff `main` vs branche : confirmer que Bakin = « 1 module + 1 arm/impl » devient vrai.
- [x] **Hors périmètre** (ne PAS toucher) : `tokenizer.rs:95/135`, `qa.rs:225`
      (`engine=="wolf"` = config par moteur, pas dispatch de fichier ; hors liste de fichiers
      de la phase). Backlog inchangé (doublons TM, ESLint 10, QA live sans glossaire, 8 lints
      clippy --all-targets).

### Docs (fin de phase)
- [x] `docs/architecture.md` : documenter les méthodes d'`Engine` + la classification comme
      point de dispatch unique.
- [x] ADR si l'abstraction est structurante (enum-méthodes vs trait — décision + rejet trait).
- [x] CHANGELOG.md (Changed) + docs/journal/ + tasks/todo.md cochés.

## ✅ Phase 6 — DRY backend + UI — TERMINÉE

> Plan §Phase 6. **Sites vérifiés sur le code réel (2026-07-02)** — voir n° de
> ligne ci-dessous. Pas un bugfix → pas de « test rouge » ; le filet est la suite
> Phase 7 (367 Rust + 2 intégration + 23 front) : **chaque refacto doit laisser le
> gate 100 % vert sans changer un seul test** (refacto = iso-comportement). Toute
> divergence de comportement observée = ce n'était pas une simple duplication →
> STOP + re-plan. Branche : `refactor/dry-phase-6`.
>
> **Ordre** : backend d'abord (1→5, un commit), puis UI (6→9, un commit), merge
> unique. Gate complet entre les deux blocs.

### Volet backend

- [x] **1. Stats recompute (dupliqué 3×)** → `core/manifest.rs::refresh_stats`.
      Le bloc `SELECT (…4 COUNT…) FROM … WHERE …` + `manifest::update_stats(…)`
      est copié-collé à `project.rs:421-450` (clé `s.id = ?`),
      `translate.rs:148-174` (clé `s.id = ?`) et `translate.rs:368-394` (clé
      `project_id`). Extraire `pub async fn refresh_stats(pool, project_id)` (best-
      effort, ne renvoie pas d'erreur) ; `update_segment`/`translate.rs:148`
      résolvent `project_id` depuis le segment d'abord (une requête triviale, ou
      variante `refresh_stats_for_segment`). **Attention** : garder le caractère
      *best-effort* (les 3 sites ignorent l'erreur) — ne pas transformer en
      `?`/propagation.
- [x] **2. Filtre glossaire pertinent (dupliqué)** → `core/glossary.rs`.
      `translate.rs:105-128` (charge `list_for_project`, filtre
      `pairs.contains(t.source_text)`, fallback par longueur si vide) est répété
      pour la passe projet. Extraire `pub async fn relevant_terms(pool,
      project_id, lang_pair, sources: &[…]) -> Vec<(String,String)>` avec la même
      logique de fallback ; les 2 passes l'appellent.
- [x] **3. Test « fichier Map ? » (dupliqué 4×)** → helper partagé.
      `project.rs:814, 886, 916` + `vx_ace/extractor.rs:435` font tous
      `file_name.starts_with("Map")` + `trim_start_matches("Map")`. Extraire
      (`engines/mod.rs` ou `utils`) `fn map_id_from_name(file_name) ->
      Option<&str>` (ou `is_map_file` + extraction du numéro) ; les 4 sites
      l'utilisent. **Vérifier** : les 3 sites de project.rs partagent-ils
      exactement la même sémantique (extension `.json` implicite) — sinon garder
      le helper minimal (booléen + id) sans forcer une unification abusive.
- [x] **4. Table `QaError` pénalité + label (match répété)** →
      `QaError::kind()`/table unique. `qa.rs:262-264` (pénalités 10/15/15),
      `qa.rs:283-330` (`label_en`/`label_fr`), et `report.rs` (labels) répètent le
      `match` par variante. Introduire une seule source (ex. `impl QaError { fn
      penalty(&self)->i32; fn label(&self, lang)->String }` déjà partiellement là)
      et faire consommer `report.rs` par ces méthodes plutôt qu'un `match`
      parallèle. **Risque moyen** : le score QA est testé (`qa.rs` tests) — le gate
      doit rester vert à l'identique (mêmes pénalités, mêmes libellés fr/en).
- [x] **5. glossaire — `SELECT` 9-col + N+1 dédup** → `core/glossary.rs`.
      Le `SELECT id, source_text, target_text, lang_pair, domain, project_id, …`
      est répété à `:116, :127, :307` → une `const GLOSSARY_COLUMNS` (ou fonction
      de mapping `FromRow`). Corriger le **N+1** de dédup (`:230-253` : un `SELECT
      COUNT(*)` par terme dans la boucle `for term in …take(50)`) par un pré-fetch
      unique des `source_text` existants (un seul `SELECT source_text WHERE
      project_id = ?` → `HashSet`) puis test en mémoire. **Iso-comportement** :
      mêmes termes insérés/ignorés qu'avant.

### Volet UI

- [x] **6. Helper « ouvrir un jeu + mapper l'erreur moteur » (dupliqué 3×)**.
      `AppToolbar.tsx:60-66` + `ProjectList.tsx:60-66, 101-107` refont le même
      `catch` → `msg.includes("could not identify game engine") ? clé A : clé B`.
      Extraire un util (`lib/openProject.ts` ou méthode de `stores/project.ts`)
      qui invoque `open_project`, mappe l'erreur en clé i18n, et laisse le
      composant afficher le toast. Ne pas déplacer l'UI (toast) dans le store.
- [x] **7. Hook `useExportToFile(...)`** → factorise `QAPanel.tsx:127-140` et
      `TMPanel.tsx:75-87` (même séquence `save()` dialog → `invoke(cmd,…)` →
      `toast.success/error`). Hook générique `(cmd, args, {successKey, errorKey})`.
- [x] **8. Source unique statut→couleur** (scopé, validé advisor) — extrait
      `lib/statusSummary.ts` (`STATUS_SUMMARY` : convention chip ✓/◎/⚠/○ partagée
      FileTree + ProjectList, classes verbatim). **PAS** fusionné avec
      `STATUS_STYLES` (badge grille cyan/or) : concepts/teintes distincts →
      fusion = changement de comportement. — **scoper d'abord** : ce sont
      potentiellement 3 échelles *différentes* (statut de segment `STATUS_STYLES`
      / `SegmentStatsBar` ; icône par *type de fichier* `FileTree fileIcon` ;
      qualité de *match TM* `MatchBadge`). N'unifier que ce qui est réellement le
      même concept (statut de segment) dans un `lib/statusStyles.ts` ; **ne pas**
      fusionner de force des palettes de concepts distincts (anti-DRY abusif).
- [x] **9. Listener `h2s://glossary/extraction-done`** (scopé, validé advisor) —
      extrait uniquement le type de payload `GlossaryExtractionDonePayload`
      (types.ts), utilisé par les 3 listeners. **PAS** fusionné en un listener
      unique : les 3 mettent à jour des états/toasts distincts → fusion = refonte
      d'architecture d'état, hors iso-comportement. — original ci-dessous :
      `useAppHandlers.ts:44`, `SegmentGrid.tsx:240`, `GlossaryPanel.tsx:248`.
      Vérifier s'ils font la *même* chose (recharger le glossaire / rafraîchir la
      grille) ; si oui, centraliser en **un** listener (probablement dans
      `useAppHandlers` ou un hook dédié) qui met à jour le store, les autres
      consommant le store. **Attention** : ne pas casser le rafraîchissement de la
      grille après extraction (comportement observable Phase 3/5).

### Vérification
- [x] Backend (1-5) : gate complet vert **sans modifier aucun test** —
      `pnpm typecheck && pnpm test && cargo clippy -- -D warnings && cargo test`.
      → 367 Rust + 2 intégration + 23 front, clippy exit 0, typecheck clean.
- [x] UI (6-9) : idem ; les tests front existants (`ProjectList`, `SegmentGrid`,
      `FileTree`, `useAppHandlers`) restés verts inchangés → iso-comportement prouvé.
      Réserve : `useExportToFile` (item 7) prouvé iso **par inspection** (QAPanel/
      TMPanel sans test) — tests panneaux relèvent de Phase 7 (hors périmètre).
- [x] `docs/architecture.md` : `manifest::refresh_stats` noté point unique de
      recalcul de stats.
- [x] CHANGELOG.md (Changed) + docs/journal/2026-07-02-dry-phase-6.md + todo cochés.
- [ ] **Hors scope** (log backlog, ne pas traiter ici) : doublons TM (index non-
      UNIQUE), `pnpm lint` cassé (ESLint 10), QA live sans glossaire, 8 lints
      `clippy --all-targets` dans le code de test.

## ✅ Phase 7 — Couverture de tests front + intégration — TERMINÉE

> Plan §Phase 7. **Vérifié sur le code réel (2026-07-02)** : `package.json` n'a
> AUCUN script `test` (CLAUDE.md documente pourtant `pnpm test` — mensonge à
> corriger par l'implémentation) ; pas de `vitest.config.ts` ; `src-tauri/tests/`
> existe mais est VIDE ; `Cargo.toml` sans feature `test` de tauri ; tous les
> modules lib.rs sont `pub` (tests d'intégration possibles sans churn).
> Branche : `feat/test-coverage`. Pas de « test rouge » ici (pas un bugfix) —
> le critère est : chaque fix des Phases 3/5 verrouillé par un test qui échouerait
> si on le révertait.

### Volet A — Infra Vitest
- [ ] `pnpm add -D vitest jsdom @testing-library/react @testing-library/jest-dom
      @testing-library/user-event`
- [ ] `vitest.config.ts` (environnement jsdom, setupFiles) — séparé de
      vite.config.ts (qui porte les réglages Tauri)
- [ ] `src/test/setup.ts` : jest-dom + init i18n (locale en) + `mockIPC`
      (`@tauri-apps/api/mocks`) + `vi.mock` de `plugin-store`/`plugin-dialog`
- [ ] `package.json` : scripts `test` (= `vitest run`) et `test:watch`

### Volet B — Tests stores (Zustand)
- [ ] `settings.ts` : défauts = `lib/constants` (gemma4:e4b) ; `saveSettings`
      persiste (mock plugin-store) ET pousse `providerConfig` dans le store llm
- [ ] `llm.ts` : `setProviderConfig` (merge partiel), `reset`, défauts = constants
- [ ] `project.ts` : `openProject` (invoke mocké) → projet ajouté + actif +
      stats fetchées ; erreur → propagée au caller
- [ ] `editor.ts` : `setActiveSegment` / `setGlossaryTerms` + sélecteurs

### Volet C — Tests composants/hooks (verrouillent les fixes Phases 3/5)
- [ ] `ProjectList` : `get_project_stats` rejette pour p1, résout pour p2 →
      la barre de p2 s'affiche quand même (allSettled, Phase 3)
- [ ] `FileTree` : fichier `⚠ M > 0` → compteurs affichés, PAS de bouton inject ;
      fichier 100 % translated → `✓ N` + bouton (Option 2, Phase 5)
- [ ] `SegmentGrid` (mock `@tanstack/react-virtual` → tous les items rendus) :
      · chargement multi-pages : mock `get_segments` paginé → N appels jusqu'à
        `total`, footer = total réel (Phase 3)
      · sélection : cocher 2 lignes → « Traduire 2 lignes » ; taper une recherche
        → bouton disparu (reset, Phase 3)
      · `update_segment` rejette → toast d'erreur `saveError` (Phase 3)
- [ ] `useAppHandlers` (renderHook) : gate export — `untranslatedCount > 0` →
      dialog `blocked`, sinon `confirm`

### Volet D — Intégration Rust `src-tauri/tests/`
- [x] `Cargo.toml` `[dev-dependencies]` : `tauri = { version = "2",
      features = ["test"] }` + `zip` (les tests d'intégration n'héritent pas
      des `[dependencies]`)
- [x] `tests/e2e_project_flow.rs` : `mock_builder().manage(AppState).build(
      mock_context(noop_assets()))` ; **appel direct** des `pub async fn`
      (`open_project`/`update_segment`/`export_project`) avec un `State` via
      `Manager::state()` — teste la couche logique, pas la sérialisation IPC ;
      `mock_context(noop_assets())` évite la dépendance à `dist/` (gitignoré)
- [x] Scénario MV : `open_project` sur la fixture réelle
      (`test/性処理係のある学校`, copie de `www/data` seule) → `update_segment`
      d'1 segment → `export_project` → le ZIP contient la traduction
- [x] Scénario Wolf CE (**verrouille Phase 1**) : `open_project` sur Honoka
      (copie `Data/BasicData`+`MapData` + stub `Game.ini`) → traduire 1 segment
      `wolf_common_events` → `export_project` → `Data/BasicData/CommonEvent.dat`
      du ZIP contient la traduction (marqueur ASCII, identique en UTF-8 et
      Shift-JIS)
- [x] Portabilité (leçon Phase 1) : `if !fixture.exists() { return; }` — skip
      propre sur un checkout sans fixtures ; isolation via `tempfile::tempdir`
      (le `.hoshi2star.json` et le `hoshi2star.zip` sont écrits DANS le tempdir →
      cleanup gratuit au `Drop`, aucune pollution des fixtures)

### Vérification
- [x] Gate étendu : `pnpm typecheck && pnpm test && cargo clippy -- -D warnings
      && cargo test` — 23 tests front + 367 unitaires Rust + 2 intégration ✅
- [x] CLAUDE.md : la ligne `pnpm test # Vitest` devient vraie (script ajouté) ;
      gate `pnpm test` ajouté à la section vérification de CLAUDE.md
- [x] docs/architecture.md : nouvelle section « Tests » (pyramide : unitaires
      inline Rust · intégration tests/ · front Vitest)
- [x] CHANGELOG.md (Added) + docs/journal/ + tasks/todo.md cochés
- [ ] Backlog (hors scope) : 8 lints `clippy --all-targets` préexistants dans
      le code de test `#[cfg(test)]` (`non_snake_case`, `if_same_then_else`) —
      non couverts par le gate mandaté (sans `--all-targets`)

## ✅ Phase 5 — Cohérence #8 (Option 2) + alignements doc/valeurs — TERMINÉE

> Plan §Phase 5 + complément §6. Décision produit #8 **déjà tranchée : Option 2**.
> **Vérifié sur le code réel (2026-07-01)** : `get_source_files` compte
> `target_text != ''` (project.rs:295) ; `get_project_stats` ignore `reviewed`
> (:461-489) ; FileTree `isComplete` gate le bouton inject (:177-178, aucun
> compteur affiché aujourd'hui) ; `SegmentStatsBar` ne rend pas `reviewed`
> (ProjectList.tsx:238-272) ; modèle par défaut divergent 3× (provider.rs:83
> `q4_K_M` · settings.ts:23 `q8_0` · llm.ts:50 `qwen3:4b`) ; `H2sError` 0
> occurrence (CONTEXT.md:195, CLAUDE.md:175) ; « skeleton » CLAUDE.md:69.
> ⚠ **Types IPC modifiés** → docs/architecture.md À METTRE À JOUR.
> Branche : `fix/status-consistency`.

### Étape A — Backend : compteurs stricts + reviewed
- [x] Helpers `fetch_source_files` / `fetch_project_stats` (`&SqlitePool`,
      commands = façades) — testables sans `tauri::State`
- [x] `translated_count` strict (`status='translated'`) + `needs_review_count`
      par fichier ; `reviewed_count` projet (somme des 4 == total)
- [x] `domain/types.rs` : 2 nouveaux champs (`#[sqlx(default)]` couvre les
      requêtes à littéraux `0 as …`, ex. export.rs:447)
- [x] Test rouge prouvé : `translated_count` retournait **4** au lieu de 1
      (4 segments, 1 par statut, tous target non vide) →
      `test_counter_semantics_one_segment_per_status` vert, fige les 4 compteurs

### Étape B — Frontend : miroir types + FileTree + barre
- [x] `lib/types.ts` : `needsReviewCount` / `reviewedCount` (doc-comments)
- [x] `FileTree.tsx` : compteurs `✓ N · ⚠ M` (verts/ambre) ; inject seulement si
      100 % strictement `translated`
- [x] `SegmentStatsBar` : segment `reviewed` bleu (`◎ N`) — somme à 100 %
- [x] Consommateurs de `translatedCount` vérifiés (grep) : FileTree (voulu),
      AppToolbar % (déjà strict via ProjectStats), project.ts toast (idem)

### Étape C — Alignements annexes
- [x] Modèle par défaut unifié sur **`gemma4:e4b`** (choix utilisateur, tag
      vérifié sur son Ollama local — remplace q4_K_M/q8_0/qwen3:4b) :
      source TS unique `lib/constants.ts` (settings.ts importait déjà llm.ts →
      un import inverse aurait créé un cycle) + miroir Rust `DEFAULT_OLLAMA_MODEL`,
      commentaires croisés. Section RunPod de CONTEXT.md non touchée (exemple
      de workflow cloud, pas le défaut de l'app).
- [x] CONTEXT.md + CLAUDE.md : convention `Result<T, String>` + thiserror par
      module actée (pas de `H2sError` global) ; « skeleton » → état réel

### Vérification
- [x] Gate : `pnpm typecheck` ✅ · `cargo clippy -- -D warnings` ✅ ·
      `cargo test` = **367 pass / 0 fail / 4 ignored** (+1)
- [x] **docs/architecture.md mis à jour** : section domain/types.rs (Option 2,
      helpers, invariant somme, UI ✓/⚠) + miroir constants.ts
- [x] CHANGELOG.md (Fixed + Changed) + docs/journal/ + tasks/todo.md cochés
- [x] Vérification visuelle en live (2 statuts basculés temporairement puis
      restaurés) : ProjectList `✓ 15496 · ◎ 1 · ⚠ 1 · ○ 0 — 15498` (somme
      exacte) ; FileTree `Map118 ✓ 44` + bouton inject vs `Map119 ✓ 559 · ⚠ 1`
      SANS bouton inject

## ✅ Phase 4 — Correctness du moteur QA (P2, faux résultats) — TERMINÉE

> Plan de `docs/audit-remediation-plan-2026-07-01.md` §Phase 4 + complément §1.2/§1.3.
> **Vérifié sur le code réel (2026-07-01)** : les 3 défauts confirmés —
> `qa.rs:134` (`'\u{FF00}'..='\u{FFEF}'` entier compté largeur 2),
> `report.rs:231-232` (`total_checked = details.len()`, le commentaire l'avoue),
> `report.rs:71` (`qa::check(..., &[], ...)`, doc de module :4 « Glossary terms are
> not applied »). `export_qa_report` vit dans `commands/export.rs:366` (pas qa.rs).
> Branche : `fix/qa-correctness`.

### Étape 1 — `qa.rs` : katakana demi-largeur comptés pleine largeur
- [x] Test rouge prouvé : `measure_line_units("ｱｲｳ")` retournait 6.0 (attendu 3.0)
- [x] Fix : `'\u{FF00}'..='\u{FFEF}'` → sous-plages EAW **F** réelles
      (`FF00–FF60` + `FFE0–FFE6`) ; gardes `｡･` == 2.0 et `Ａ￥` == 4.0

### Étape 2 — `report.rs` : dénominateur « X / Y vérifiés » toujours X == Y
- [x] `collect_qa_details` → `(total_checked, details)` (total = lignes avant
      filtrage `score < 100`)
- [x] `generate_qa_html(…, total_checked, lang)` — plus de `details.len()`
- [x] `export_qa_report` propage le tuple
- [x] Test : « 1 segments with errors / 10 checked » avec 1 détail + total 10

### Étape 3 — `report.rs:71` : check glossaire mort — Option A (câblé)
- [x] `collect_qa_details(pool, project_id, terms)` — termes injectés
- [x] `export_qa_report` charge `glossary::list_for_project(…, "ja-en")` →
      map `(source_text, target_text)`
- [x] Doc de module `report.rs:1-6` mise à jour
- [x] Test rouge intégration prouvé (pool SQLite via `db::pool::init` + tempfile,
      pattern déjà présent dans pool.rs — pas de nouvelle infra nécessaire) :
      2 segments traduits, s1 viole ハルカ→Haruka → AVANT fix : `got: []` ;
      APRÈS : `GlossaryMismatch` sur s1 + `total_checked == 2` + `details.len() == 1`

### Hors scope (loggé → Backlog)
- `qa_check_segment` (`commands/qa.rs:37`) et `update_segment`
  (`commands/project.rs:388`) passent toujours `&[]` → le QA live ne détecte pas
  les GlossaryMismatch (cohérent : le panneau live n'affiche pas de stat glossaire).

### Vérification
- [x] Tests rouges prouvés avant fix (kana 6.0≠3.0 · glossaire `got: []`)
- [x] Gate : `pnpm typecheck` ✅ · `cargo clippy -- -D warnings` ✅ ·
      `cargo test` = **366 pass / 0 fail / 4 ignored** (+3)
- [x] docs/architecture.md : non touché (signatures internes core/, aucun type
      IPC ni commande modifiés)
- [x] CHANGELOG.md (Fixed) + docs/journal/ + tasks/todo.md cochés

**Phase 4 : livrée — largeur kana correcte, dénominateur réel, GlossaryMismatch
vivant dans le rapport (premier test d'intégration DB du module report).**

## ✅ Phase 3 — Robustesse des promesses UI (P1, silencieux) — TERMINÉE

> Plan de `docs/audit-remediation-plan-2026-07-01.md` §Phase 3 + complément §1.4/§1.5.
> **Vérifié sur le code réel (2026-07-01)** : les 4 défauts sont confirmés aux lignes
> de l'audit. Fait nouveau : `get_segments` retourne DÉJÀ `total` (vrai `COUNT`,
> `project.rs:332`) et `PaginatedSegments.total` existe côté TS (`types.ts:72`) —
> l'UI l'ignore. Branche : `fix/ui-promise-robustness`.
> **Contrainte** : aucun test front n'existe (Vitest = Phase 7) → vérification
> manuelle via `pnpm tauri dev` + gate typecheck/clippy/tests (le plan l'acte).

### Étape 1 — `ProjectList.tsx:44-51` : stats tout-ou-rien
- [x] `Promise.allSettled` : un `get_project_stats` en échec ne rejette plus le lot

### Étape 2 — `SegmentGrid.tsx:230-244` : `handleSave` rejet non géré
- [x] try/catch autour de `invoke("update_segment")` + `toast.error`
- [x] i18n : clé `segmentGrid.saveError` dans `en.json` + `fr.json`

### Étape 3 — `SegmentGrid.tsx` : sélection → mauvais segments après filtrage
- [x] `getRowId: (row) => row.id` → `rowSelection` keyée par id de segment ;
      `selectedIds` sans mapping par index — désigner un autre segment devient impossible
- [x] Reset de `rowSelection` sur changement de `qaFilter`/`searchQuery`

### Étape 4 — `SegmentGrid.tsx:66-77` : plafond 5000 lignes silencieux
> Décision validée : **chargement par pages successives** (pageSize 2000).
- [x] `loadSegments` : boucle jusqu'à `total` + garde d'annulation `loadSeqRef`
      (un load périmé n'écrase ni `segments` ni `isLoading` du fichier suivant)
- [x] Footer/`totalCount` : corrects une fois le fichier complet (`isLoading` couvre
      le chargement)
- [x] Doc-comment `get_segments` aligné (pages successives de 2000)

### Étape 5 — `QAPanel.tsx` / `TMPanel.tsx` : dialogues `save()` non gardés
- [x] `await save(...)` dans try/catch → toast `exportError` existante (les 2 panneaux)

### Vérification
- [x] Gate : `pnpm typecheck` ✅ · `cargo clippy -- -D warnings` ✅ · `cargo test`
      = 363 pass (Rust intact hors doc-comment)
- [x] **Vérification manuelle via MCP Tauri** (`pnpm tauri:linux`, projet réel MV) :
      · ProjectList : stats des 2 cartes affichées (✓ 15498 / ✓ 22803) — allSettled OK
      · Map119.json (561 segments) chargé ENTIER, footer « 561 segments », filtré « 6 / 561 »
      · Boucle multi-pages prouvée sur le protocole réel : 6 pages × 100 → 561 ids uniques
      · Sélection : 2 lignes cochées → « Traduire 2 lignes » ; recherche modifiée →
        bouton disparu + 0 case cochée (reset OK)
      · handleSave : édition « Sandwich » → « Sandwich. » persistée (updatedAt bump),
        puis restaurée à l'identique (TM nettoyée, cf. découverte ci-dessous)
- [x] docs/architecture.md non touché (aucun changement d'architecture)
- [x] CHANGELOG.md (Fixed) + docs/journal/ + tasks/todo.md cochés

**Phase 3 : livrée — 5 fixes UI, vérifiés en live sur l'app dev pilotée via MCP.**

### Découvertes hors scope loggées (→ Backlog)
- 🐛 **TM : doublons à chaque re-sauvegarde** — `tm.rs::insert` utilise
  `INSERT OR REPLACE` mais le PK est un UUID neuf et `idx_tm_hash_lang` n'est PAS
  UNIQUE → le REPLACE ne remplace jamais, une ligne s'ajoute à chaque save du même
  segment (observé en live : 2 entrées pour le même `source_hash`). Fix : migration
  index UNIQUE (source_hash, lang_pair) + dédup des données existantes + upsert.
- 🐛 **`pnpm lint` cassé** (pré-existant, vérifié sur arbre propre) : ESLint 10
  exige `eslint.config.js` (flat config), le repo a encore `.eslintrc.*`.
- Phase 7 : tests front à écrire pour les 4 fixes (ProjectList allSettled,
  handleSave erreur, sélection/filtre, chargement complet multi-pages)

## ✅ Phase 2 — Éliminer les paniques des parsers binaires Wolf (P1, crash) — TERMINÉE

> Plan de `docs/audit-remediation-plan-2026-07-01.md` §Phase 2 + complément §2.
> **Vérifié sur le code réel (2026-07-01)** : tous les sites sont encore aux lignes de
> l'audit (le code n'a pas bougé). Tous les champs `DxFileEntry` sont `u64` attaquant
> (`legacy_xor.rs:62-66`) → les casts `as usize` + additions brutes peuvent réellement
> wrapper même en 64-bit. Branche : `fix/wolf-parser-panics`.
> **NE PAS toucher** : les `catch_unwind` d'`extractor.rs` (garde-fou voulu).
> Convention erreur : réutiliser `DecryptorError::HeaderTooShort` (déjà utilisé pour
> toutes les bornes voisines :555/582/1185/1196…) — pas de nouveau variant.

### Étape A — Racine : allocation Huffman non bornée (`legacy_xor.rs:731`)
- [x] Test rouge : `huffman_decode(&[0xFF;16])` (orig_bits=64, orig_size≈u64::MAX) →
      panique « capacity overflow » prouvée
- [x] Fix : const `MAX_DECODE_OUTPUT = 1 GiB` (partagée Huffman/LZ) → `return None`
      (les appelants mappent déjà `None` → `Err`)
- [x] Même classe : `lz_decode` — `dest_size` plafonné (rouge prouvé : retournait
      `Some(vec 4 GiB)`)

### Étape B — Slice TOC non bornée (`legacy_xor.rs:591`)
- [x] Test rouge : `make_v5_archive` + helper `patch_v5_toc` (XOR symétrique) avec
      `name_offset=0xFFFF` → panique OOB prouvée
- [x] Fix : `usize::try_from(name_offset)` + borne `ns ≤ toc_data.len()` → `Err(HeaderTooShort)`

### Étape C — Slices Huffman v8 (`legacy_xor.rs:1214/1216` + `:1267/1269`)
- [x] Tests rouges : appels directs `extract_v8_huffman_only`/`assemble_v8_lz_stream`
      avec blob `[0u8;16]` (décode vers tampon VIDE) + `unpacked/press_sz > 2*huff_kb`
      → panique « range end index 1024 out of range for slice of length 0 » prouvée ×2
- [x] Fix : `decoded.len() < huff_kb * 2` → `Err(HeaderTooShort)` dans les 2 fonctions

### Étape D — Tier arithmétique : additions d'offsets en `checked_add`
- [x] `:541` (`saturating_add`), `:554` (check u64 `index_offset+index_size` avant cast),
      `:580` (checked u64 + `try_from`), `:1387` (idem v8)
- [x] Sites frères mêmes fonctions : `:1185`, `:1204-1205`, `:1238`, `:1257-1258`,
      `:1396`, `:1419` + casts `u64→usize` non tronquants (`unp`, `pz`, `hz`) +
      `key_offset + huff_sz` (u64) en `checked_add`
- [x] **Ajouts vérifiés hors liste audit (même classe, même chemin de dispatch v8)** :
      `read_original_name` (`:1016/:1020` saturating), `build_per_file_key_str`
      (`:1049` saturating), `find_parent_dir` (`:992` `checked_mul`+`checked_add`,
      skip du dir forgé)
- [x] Tests rouges : v6 `data_offset=u64::MAX` via `patch_v6_toc` (panique add overflow
      prouvée) + `extract_v8_huffman_only(file_start≈usize::MAX)` (panique add prouvée).
      `:554` non testable en rouge (champs v5/v6 u32 → pas de wrap 64-bit) → défensif.

### Étape E — `v3_format/map.rs:355/369` : multiplication tiles non vérifiée
- [x] Test rouge : dump d'une map valide + patch `width=0x4000_0000, height=4` →
      « attempt to multiply with overflow » prouvé (map.rs:369)
- [x] Fix : `tile_data_len(width,height,layer_cnt)` en `checked_mul` u64 → nouveau
      variant `V3FormatError::TileSizeOverflow` ; branche utf8 : `read_bytes` (borné,
      sans alloc) AVANT `Vec::with_capacity(tile_len)`

### Étape F — `dat_parser.rs` : allocations pilotées par l'entrée
- [x] `read_bytes` : refuse `n > bytes restants` AVANT `vec![0u8; n]` (couvre aussi
      `n*4`/`cnt*4`, passés en `saturating_mul`)
- [x] Helper `bounded_cap(cursor, count)` (`count.min(remaining/4)`) appliqué aux 5
      `with_capacity` attaquants : `parse_project` ×3 + `parse_dat_types` `fields_size`
      + `data_count`
- [x] Tests rouges : `read_bytes(n=usize::MAX)` → « capacity overflow » prouvé ;
      `type_count=u32::MAX` → **SIGABRT alloc 309 Go** prouvé (tuait tout le process !) ;
      `fields_size=u32::MAX` → **SIGABRT alloc 137 Go** prouvé

### Vérification (gate obligatoire)
- [x] 11 tests rouges prouvés AVANT fix (9 paniques + 2 SIGABRT), 11/11 verts après
- [x] Non-régression fixtures réelles : `test_real_honoka_*` (v2 + archive v8 DXA) +
      `test_real_inko_*` (v3.5) tous verts
- [x] Gate : `pnpm typecheck` ✅ · `cargo clippy -- -D warnings` ✅ (seul warning :
      toolchain `pclmul` pré-existant, hors code) · `cargo test` = **363 pass / 0 fail / 4 ignored**
- [x] CHANGELOG.md (Fixed)
- [x] docs/architecture.md : PAS touché — aucun changement d'architecture (fns internes
      durcies ; seul ajout : variant interne `V3FormatError::TileSizeOverflow`, pas un type IPC)
- [x] docs/journal/ : entrée de session
- [x] tasks/todo.md : coché

**Phase 2 : livrée — 11 sites de panique/abort éliminés (legacy_xor.rs, v3_format/map.rs,
dat_parser.rs), `catch_unwind` d'extractor.rs intacts, prouvé rouge→vert site par site.**

## ✅ Phase 1 — Réinjecter les Common Events Wolf à l'export (P0, perte de données) — TERMINÉE

> Plan de `docs/audit-remediation-plan-2026-07-01.md` §Phase 1. Vérifié sur le code réel
> (2026-07-01). Le bug est confirmé : `inject_all_to_memory`/`inject_all` (injector.rs:483/521)
> ne matchent que `MapData`/`Database`, `_ => {}` (514/568). Aucun `inject_common_events`.

### Étape 1 — Verify read-only + test rouge (priorité absolue)
- [x] Pas de fixture Wolf réelle dans ce checkout (`test/性処理係のある学校` = MV/MZ) →
      reproduction via CE v3 synthétique (`CommonEventsV3::synthetic_messages`), pas de fixture chiffrée
- [x] Test **rouge** via le **regroupement** (extract→`injection_bucket`→`inject_all_to_memory`)
      avec 2 event_names → prouvé rouge (compile-fail sur `injection_bucket` absent, puis
      assert perte). `test_common_events_translations_survive_export`

### Contraintes de correction (silencieuses si ratées — cf. advisor)
- [x] **C1 — Bucket unique** : `injection_bucket()` collapse tous les `CommonEvents/*` en
      `COMMON_EVENTS_BUCKET = "CommonEvents/CommonEvent"`. Les 2 sites de bucketing d'`export.rs`
      (`collect_wolf_zip_entries` + `debug_inject_file`) l'utilisent. Clé complète conservée.
- [x] **C2 — Parité de clés** : injecteur régénère `CommonEvents/{event_name}/{event_idx}/{cmd_idx}`
      (+`/choices/{choice_idx}`), pas de `pages`, `event_name = event.event_name()` (v2) / `event.name` (v3).
- [x] **C3** : test rouge passe par `injection_bucket` + `inject_all_to_memory` (2 event_names).

### Implémentation
- [x] `inject_common_events(bytes, translations, version)` : v2 → `common_events_parser::parse_bytes`
      (catch_unwind) + `patch_common_events_strings` (remplacements ordonnés, dst=src si non traduit) +
      `splice_wolf_strings` (splice partagé extrait de `patch_mps_strings`). v3 → `inject_common_events_v3`
      (`is_lz4_v3` → `decompress` → `CommonEventsV3::parse` → mute `string_args` → `dump` → `recompress`).
- [x] Arm `"CommonEvents"` dans **`inject_all_to_memory` ET `inject_all`** → `load_common_event_bytes`,
      écrit/emet `Data/BasicData/CommonEvent.dat`.
- [x] `export.rs` : les 2 sites routent via `injection_bucket`.

### Vérification
- [x] **v3 (Inko)** vert : `test_common_events_translations_survive_export` (2 CE, both survive) via `CommonEventsV3::synthetic_messages`
- [x] **v2 (Honoka)** vert : fixture v2 synthétique construite à la main (`make_v2_common_event_dat`) →
      `test_v2_common_event_fixture_extracts` + `test_v2_common_events_translations_survive_export` (2 CE) +
      `test_v2_common_events_identity` (byte-exact, doublons → pas de dé-alignement du splice)
- [x] `test_injection_bucket_collapses_common_events`
- [x] **Vérif fichiers réels** (fixtures fournies par l'utilisateur, renommées en `Densyanai_Inko_ver2.0` /
      `月咲流ホノカver1.03` pour matcher les 13 refs de tests existantes) :
      `test_real_honoka_common_events_inject` (v2 : identity byte-exact + trad round-trip) +
      `test_real_inko_common_events_inject` (v3.5 : identity payload décompressé + trad round-trip)
- [x] Gate : `pnpm typecheck` ✅ · `cargo clippy -- -D warnings` ✅ · `cargo test` = **352 pass / 0 fail / 4 ignored**
- [x] `docs/architecture.md` : section `wolf/injector.rs` + date 2026-07-01
- [x] CHANGELOG.md (Fixed)

**Phase 1 : livrée — v2 (Honoka) ET v3 (Inko) vérifiés sur fixtures synthétiques ET sur les jeux réels. Commitée + mergée sur `main` (2 commits + merge --no-ff).**

### Résolu par les fixtures fournies
- [x] Les 2 tests `test_real_inko_*_round_trip` qui paniquaient (fixture Inko absente) passent maintenant
      (fixtures présentes). Note portabilité : ils paniquent toujours (au lieu de skip) sur un checkout sans
      fixtures — aligner sur `if !path.exists() { return; }` reste un petit nettoyage optionnel.

---

## Complétées (session 2026-06-17/18)

- [x] Wolf extractor skip filters : `X[`/`zz` events, `自動ｼｽﾃﾑ初期化` DB, `@N\n` tokenizer (317 tests)
- [x] `extract_wolf_speakers` Tauri command + bouton "Speakers" dans GlossaryPanel
- [x] `SourceFile.translated_count` / `total_count` + requête SQL `get_source_files`
- [x] `debug_inject_file` Tauri command (Wolf, complétude enforced)
- [x] `scan_font_status` Tauri command + `FontSizeDialog` + `\f[N]` prefix management
- [x] `export_project` étendu avec `fontSize` / `replaceExisting`
- [x] Bouton Debug Inject par fichier dans `FileTree.tsx` (hover, `FlaskConical`)
- [x] Docs CHANGELOG + ROADMAP + architecture.md mis à jour

## Complété — MV/MZ placeholder codes custom + GameTitle (2026-06-19)

- [x] 1. `tokenizer.rs` — Groupe F : `\FF[...]`, `\F[...]`, `\AA[...]` dans RE_MVMZ + RE_MZONLY
- [x] 2. `mv_mz/extractor.rs` — GameTitle + " by Hoshi2Star"
- [x] 3. 6 tests Groupe F + test GameTitle mis à jour
- [x] 4. 323 tests ✓ · clippy ✓

## Complété — Debug Extraction universelle (2026-06-19)

Objectif : rendre le bouton Bug (debug dump JSON) disponible pour tous les moteurs,
pas uniquement Wolf. Le JSON doit avoir un format unifié pour que Claude puisse
l'analyser et identifier ce qui mérite traduction vs ce qui peut être skippé.

- [x] 1. `commands/project.rs` — ajouter `debug_dump_segments` générique (dispatch par moteur)
- [x] 2. `commands/project.rs` — supprimer `debug_dump_wolf_segments` (remplacé)
- [x] 3. `lib.rs` — remplacer `debug_dump_wolf_segments` par `debug_dump_segments`
- [x] 4. `AppToolbar.tsx` — retirer la condition `engine === "wolf"`, appeler `debug_dump_segments`
- [x] 5. Vérification : cargo clippy ✓ · pnpm typecheck ✓

## En cours — Stats de segments (2026-06-26)

Trois emplacements : toast post-extraction · barre dans ProjectList · % dans toolbar pill.

- [x] 1. Rust `domain/types.rs` — ajouter `translated_count` + `needs_review_count` à `ProjectStats`
- [x] 2. Rust `commands/project.rs` — étendre la query SQL de `get_project_stats` (5 sous-requêtes)
- [x] 3. TS `lib/types.ts` — ajouter interface `ProjectStats` partagée
- [x] 4. TS `stores/project.ts` — ajouter `activeProjectStats`, fetch après open, toast si `!wasRestored`
- [x] 5. TS `useAppHandlers.ts` — supprimer interface locale, importer depuis `lib/types`
- [x] 6. TS `AppToolbar.tsx` — afficher `37%` dans la pill projet
- [x] 7. TS `ProjectList.tsx` — fetch stats par carte, afficher barre + compteurs
- [x] 8. i18n `en.json` + `fr.json` — ajouter clé `project.extracted`
- [x] 9. Vérification : `cargo clippy` ✓ · `pnpm typecheck` ✓

## En cours — Tokenizer Groupe G + \# (2026-06-26)

- [x] 1. `tokenizer.rs` — ajouter Groupe G (`\\n<[^>]+>`) dans RE_MVMZ et RE_MZONLY
- [x] 2. `tokenizer.rs` — ajouter `\#` (échappé) dans Groupe B des deux regex
- [x] 3. `tokenizer.rs` — 4 tests : tokenize `\n<Name>`, tokenize `\#`, round-trip, pas de conflit avec `\n[N]`
- [x] 4. Vérification : 27 tests ✓ · clippy ✓

## Complété — Export ZIP (2026-06-26)

- [x] 1. `Cargo.toml` — ajouter crate `zip = "2"`
- [x] 2. `engines/mv_mz/injector.rs` — ajouter `inject_to_bytes(raw_json, pairs) -> Vec<u8>`
- [x] 3. `engines/wolf/injector.rs` — ajouter `inject_all_to_memory(...) -> Vec<(String, Vec<u8>)>`
- [x] 4. `commands/export.rs` — refactorer `export_project` → zip `hoshi2star.zip` + `collect_wolf_zip_entries` + `write_zip`
- [x] 5. `commands/export.rs` — return type `Result<String, String>` (retourne le chemin zip)
- [x] 6. `useAppHandlers.ts` — `invoke<string>`, toast description = chemin zip
- [x] 7. i18n `en.json` + `fr.json` — messages mis à jour (ZIP / hoshi2star.zip)
- [x] 8. Vérification : clippy ✓ · typecheck ✓ · 336 tests ✓ (2 échecs pré-existants Inko)

## Complété — Font size MV/MZ (\\FS[N]) (2026-06-27)

`\FS[N]` MZ-natif (Groupe C). `\f[N]` Wolf-only. Dialog gate wolf/mv_mz.
Hint affiché pour mv_mz : nécessite MZ ou plugin messages pour MV.
clippy ✓ · typecheck ✓ · 336 tests ✓ (2 Inko pré-existants)

- [x] 1. `export.rs` — `RE_FONT_PREFIX_MVMZ` + `engine: String` dans `FontScanResult`
- [x] 2. `export.rs` — `apply_font_prefix(text, n, replace, engine)` engine-aware
- [x] 3. `export.rs` — `scan_font_status` fetch engine depuis DB, retourne engine
- [x] 4. `export.rs` — `persist_font_size(…, engine)` passe engine à apply
- [x] 5. `export.rs` — `export_project` fetch engine avec game_path
- [x] 6. `export.rs` — `debug_inject_file` utilise engine (plus `_engine`)
- [x] 7. `lib/types.ts` — `engine: string` dans `FontScanResult`
- [x] 8. `useAppHandlers.ts` — gate wolf/mv_mz uniquement
- [x] 9. `FontSizeDialog.tsx` — `code` calculé, hint mv_mz
- [x] 10. `en.json` + `fr.json` — `{{code}}` interpolé + clé `hintMvMz`
- [x] 11. Vérification ✓

## En cours — Externalisation des prompts LLM vers TOML

Objectif : sortir les prompts hardcodés de `provider.rs` et `glossary.rs` vers des
fichiers `.toml` embarqués à la compilation via `include_str!()`. Structure dossier
dès maintenant pour accueillir les langues cibles futures sans refactor.

**Architecture :**
```
src-tauri/prompts/
  translate/
    default.toml    ← fallback générique (ja→en aujourd'hui)
  glossary/
    default.toml    ← fallback générique
```
Quand on ajoutera FR : créer `translate/fr.toml` + bras `"fr"` dans le `match`.

**Variables dans les templates :**
- `translate/default.toml` : `{{source_lang}}` / `{{target_lang}}` / `{{glossary}}` / `{{segments}}`
- `glossary/default.toml` : `{{target_lang}}` / `{{source_list}}`
- Les valeurs sont des noms complets (`"Japanese"`, `"English"`) — pas des codes courts (`"ja"`, `"en"`)
  → `translate.rs` conserve `"ja"`/`"en"` en interne ; `prompts.rs` expose `lang_code_to_name()`

**Fichiers créés :**
- `src-tauri/prompts/translate/default.toml`
- `src-tauri/prompts/glossary/default.toml`
- `src-tauri/src/llm/prompts.rs`

**Fichiers modifiés :**
- `src-tauri/Cargo.toml` — ajouter `toml = "0.8"`
- `src-tauri/src/llm/mod.rs` — `pub mod prompts`
- `src-tauri/src/llm/provider.rs` — remplacer `format!()` hardcodé
- `src-tauri/src/core/glossary.rs` — remplacer strings hardcodées

- [x] 1. `Cargo.toml` — ajouter dépendance `toml = { version = "0.8", features = ["parse"] }`
- [x] 2. Créer `src-tauri/prompts/translate/default.toml` — system + user avec variables ci-dessus
- [x] 3. Créer `src-tauri/prompts/glossary/default.toml` — system + user avec variables ci-dessus
- [x] 4. Créer `src-tauri/src/llm/prompts.rs` :
         · `PromptTemplate { system, user }` + `render(part, vars)`
         · `fn lang_code_to_name(code: &str) -> &str` (`"ja"` → `"Japanese"`, `"en"` → `"English"`, …)
         · `LazyLock` `TRANSLATE_DEFAULT` + `GLOSSARY_DEFAULT`
         · `fn translate_for(target_lang: &str) -> &'static PromptTemplate` (fallback default)
         · `fn glossary_for(target_lang: &str) -> &'static PromptTemplate` (idem)
- [x] 5. `src-tauri/src/llm/mod.rs` — ajouter `pub mod prompts`
- [x] 6. `src-tauri/src/llm/provider.rs` — appeler `prompts::translate_for(&context.target_lang)`,
         passer `lang_code_to_name(source_lang)` et `lang_code_to_name(target_lang)` au `render()`
- [x] 7. `src-tauri/src/core/glossary.rs` — appeler `prompts::glossary_for(lang_target)`,
         passer `lang_code_to_name(lang_target)` au `render()`
- [x] 8. Vérification : clippy ✓ · 343/349 tests ✓ (2 échecs Inko pré-existants, 4 ignored)

## Backlog

- [ ] 🐛 TM : doublons à chaque re-sauvegarde (`tm.rs::insert` REPLACE inopérant,
      index non-UNIQUE) — découvert Phase 3, cf. section Phase 3
- [ ] 🐛 `pnpm lint` cassé : migrer `.eslintrc.*` → `eslint.config.js` (ESLint 10)
- [ ] QA live : câbler les termes glossaire dans `qa_check_segment` +
      `update_segment` (aujourd'hui `&[]` — le rapport les applique depuis Phase 4)
- [ ] Anneaux de progression par fichier dans FileTree (FileTree rings) — `translated_count`/
      `total_count` maintenant disponibles; rend la tâche dormante Tenmon réalisable
- [ ] Documentation workflow WolfX (pré-étape UberWolf) dans `docs/engines.md` +
      message UI quand `PossibleWolfX` est détecté (ROADMAP F5)
- [ ] Recrutement beta testeurs (Discord fan-trad / F95zone) — ROADMAP F3
- [ ] Diff-aware merge (`core/diff.rs`) — ROADMAP F4

### Plan détaillé — RW : extraction et réinjection Wolf v2/v3 (2026-10-26, Claude) — PLAN VALIDÉ

**Date :** 2026-10-26 — Claude. **Statut :** PLAN VALIDÉ le 2026-10-26 (D1 = a, D2 = a, D3, D4 oui, D5 = a, D6 oui). Preuves locales (hors Git) : `RAE-PLAN/wolf/`, `RAE-PLAN/essais/RESULTATS.md`.

#### Base de preuves

- Prototypes Python validés hors jeu **et en jeu** (`RAE-PLAN/wolf/_outils/`, résultats dans `RAE-PLAN/essais/RESULTATS.md`) :
  - **Wolf 2.23** : archive DXA v6, aller-retour identique octet par octet ; 19/19 cartes et `CommonEvent.dat` v2 réécrits à l'identique ; archive reconstruite **acceptée en jeu** (W3) ;
  - **Wolf 3.658** : archive DXA v8 (chiffrement 314) ; 71/71 fichiers v3 réécrits à l'identique ; archive reconstruite, table non compressée, **acceptée en jeu** (V3, en français et en anglais).
- **Ordre de chargement vérifié en jeu (v2 et v3) : l'archive passe avant les fichiers libres** (W1, V1). L'export actuel de Hoshi2Star (fichiers traduits libres dans un zip) est donc **sans effet** sur un jeu livré en archive.
- Défauts de notre Rust actuel, tous vérifiés par lecture :
  1. signature v6 chiffrée refusée (`decrypt/legacy_xor.rs:195-204`, `:518-520`) ;
  2. LZ refusé en v6 (`:573-576`) ;
  3. chaîne-clé « v3.14 » absente (`:967-968`), le jeu étant pris pour du WolfX ;
  4. cartes v2 et v3 non compressées refusées (`extractor.rs:508-513`) ;
  5. version détectée « v2 » à partir de la page de codes 932 (`detector.rs:282-285`) ;
  6. échecs d'archive ignorés en silence (`extractor.rs:215-217`) ;
  7. lecture de la source dans `Data/` avant l'archive (`extractor.rs:337-365`, `738-750`), alors que le jeu lit l'archive en premier ;
  8. aucun contrôle après écriture Wolf ;
  9. `unknown1` des `.dat` toujours écrit à 0 (`injector.rs:285-295`).

#### Comportement visé (utilisateur)

- **Ouverture :** un jeu Wolf v2 ou v3 livré en archive, ou en `Data/` libre, s'ouvre et ses textes sont extraits. Une archive illisible (nouveau chiffrement 3.31+, WolfX, Pro, clé inconnue) donne **un message clair**, jamais un projet vide en silence.
- **Export :** le zip contient **l'archive reconstruite** sous le même nom (`Data.wolf`, ou `Data/*.wolf` selon le jeu) avec les fichiers traduits ; on l'extrait par-dessus le jeu, et le jeu s'affiche traduit. Si le jeu est en fichiers libres, le zip contient les fichiers libres traduits, comme aujourd'hui.
- **Contrôles :** chaque fichier produit est relu ; toute différence hors des textes prévus refuse l'export, avec la liste des écarts (même règle que la porte d'intégrité MV/MZ).

#### Décisions à valider

- **D1. Format de livraison.**
  - (a) **recommandé** : zip avec l'archive reconstruite. Simple et dans la continuité de l'export actuel ; le zip peut peser de 200 Mo à 2 Go (archive déjà compressée, stockée sans recompression) ;
  - (b) dossier « copie complète du jeu » traduit, à côté du jeu.
- **D2. Nouveau chiffrement 3.31+, WolfX et Pro.**
  - (a) **recommandé** : refus à l'ouverture, avec un message qui explique le déchiffrement préalable par UberWolf ;
  - (b) lecture native (lourd ; licence DxLib à vérifier).
- **D3. Accents.**
  - v2 (Shift-JIS) : **remplacer les lettres accentuées par leur lettre de base** à l'export (é → e), avec le compte des lignes touchées dans le rapport. **Recommandé.** Aujourd'hui l'export échoue au premier accent.
  - v3 (UTF-8) : écrire les accents tels quels. Le rapport avertit que les polices du jeu peuvent ne pas les avoir (vu sur le jeu 3.658). Une option « replier les accents » par projet viendra plus tard.
- **D4. Archive reconstruite :** fichiers modifiés stockés **non compressés** ; v6 : alignement d'origine conservé ; v8 : fichiers modifiés ajoutés en fin de données, table écrite non compressée (drapeau DxLib « pas de compression d'en-tête »). C'est exactement ce qui a été validé en jeu. **Recommandé : oui.**
- **D5. Tests sur vrais jeux :** les jeux ne vont jamais dans Git.
  - (a) **recommandé** : tests sur fichiers synthétiques dans le gate cloud ;
  - plus des tests `#[ignore]` pointés par une variable d'environnement (`H2S_WOLF_GAMES=…`), que tu lances toi-même en local avec peu de cœurs (`cargo test -j 2 -- --ignored wolf_real`), quand tu le décides.
- **D6. Où travailler :** **worktree séparé** `../Hoshi2Star-rw`, branche `feat/rw-wolf` depuis `main`. Le dépôt principal et les worktrees Codex ne sont pas touchés. Commits locaux ; push seulement avec ton accord, pour le gate cloud. **Recommandé : oui.**

#### Étapes

Chaque étape : code, tests unitaires sur fichiers synthétiques, journal. Pas de build local ; gate en cloud après push autorisé.

**RW-1 — Lecture des archives** (`engines/wolf/decrypt/legacy_xor.rs`), effort M. — **[~] code et tests écrits (`5d90131`), non compilés ; gate cloud non exécuté.** Fait en plus : noms et chemins réels v6 (table de noms DxLib + arbre des dossiers, comme v8).
- v6 : chercher la clé sur l'en-tête chiffré dès l'octet 0, puis contrôler `DX` + version **après** déchiffrement ; garder la voie actuelle pour les en-têtes en clair.
- v6 : décompression LZ des fichiers compressés (réutiliser `lz_decode` de v8).
- v8 : lire la version de chiffrement (16 bits hauts de `Flags`) ; choisir la chaîne-clé publique selon cette version (« v2.225 », « v3.00 », « v3.14 ») plutôt qu'au hasard ; version ≥ 331 et hors Pro ⇒ nouvelle erreur `UnsupportedWolfCrypt { crypt_version }` ; 0x64/0xC8 et ≥ 1000 ⇒ WolfX/Pro (erreur existante).
- Constantes de format (chaînes-clés) avec attribution (UberWolf, MIT).
- Tests : archives synthétiques v6 (signature chiffrée, LZ) et v8 (« v3.14 ») fabriquées dans les tests à partir de l'écrivain RW-3 ou d'octets construits à la main.

**RW-2 — Détection de version** (`engines/detector.rs`), effort S.
- Ne plus déduire la version de la page de codes de l'archive.
- Utiliser, dans l'ordre : la version de chiffrement v8, l'octet UTF-8 (`0x55`) des signatures de cartes et de `.dat`, l'en-tête de `SysDatabase.dat`.

**RW-3 — Écrivain d'archive** (nouveau `engines/wolf/decrypt/writer.rs`), effort M.
- `rebuild_v6(archive, remplacements)` et `rebuild_v8(archive, remplacements)`, selon les prototypes `dxa6.py` et `dxa8.py`.
- Tests : reconstruction sans changement identique octet par octet (v6) ; relecture conforme de tous les fichiers (v6, v8) ; fichier remplacé relu identique.

**RW-4 — Cartes et événements communs non compressés** (`engines/wolf/v3_format/`), effort M.
- Accepter les cartes de version < 0x65 (v2 Shift-JIS, ou v3 0x64 UTF-8) et `CommonEvent.dat` hors 0x93/0xCC : même modèle plat, sans LZ4 ni bloc v3.5 (`wolf_v2.py` / `wolf_v3.py`).
- Retirer les refus `UnsupportedWolfFormat` correspondants (`extractor.rs`, `injector.rs`).
- Tests : aller-retour octet par octet sur fichiers synthétiques v2 et 0x64 ; injection d'un 101 et d'un 102 puis relecture.

**RW-5 — Source et livraison** (`extractor.rs`, `commands/export.rs::collect_wolf_zip_entries`), effort M.
- Lecture de la source : l'archive d'abord quand elle existe (comme le jeu), les fichiers libres ensuite.
- Nouveau `WolfDeliveryPlan` : pour chaque archive qui contient au moins un fichier traduit ⇒ archive reconstruite (RW-3) dans le zip, au même chemin ; fichiers sans archive ⇒ fichiers libres.
- Les échecs d'archive remontent en erreur visible, plus de `continue` silencieux.

**RW-6 — Contrôles après écriture** (nouveau `engines/wolf/integrity.rs`), effort S–M.
- Reparser chaque `.mps`, `CommonEvent.dat` et `.dat` produit ; vérifier que seules les chaînes prévues ont changé ; relire l'archive reconstruite (liste des fichiers, contenu des fichiers non modifiés, fichiers traduits).
- Écart ⇒ `IntegrityIssue` et export refusé (dialogue existant).
- Corriger au passage `unknown1` des `.dat` (valeur d'origine conservée).

**RW-7 — Encodage et chemins** (`engines/wolf/encoding.rs`), effort S.
- D3 : repli des accents en v2, avec le compte des lignes touchées.
- Avertissement si le chemin du jeu ou de l'export contient un caractère hors Shift-JIS : un jeu v2 n'y démarre pas, constaté en jeu.

**RW-8 — Interface**, effort S.
- Messages fr/en pour les nouvelles erreurs.
- Lignes du rapport d'export : archive reconstruite, accents repliés, avertissement de police v3.

**RW-9 — Vérification**, effort S.
- Gate cloud.
- Tests réels `--ignored` (D5).
- **Essai en jeu de l'export produit par l'app** sur les jeux 2.23 et 3.658, avec la même fiche que les prototypes.

**Ordre :** RW-1, RW-2, RW-4, RW-3, RW-6, RW-5, RW-7, RW-8, RW-9. Effort total : L.

#### Hors périmètre

- Textes en image (menu titre du jeu 3.658).
- Polices de remplacement pour les accents en v3.
- Lecture native du nouveau chiffrement 3.31+.
- Commandes Wolf autres que 101/102 (122, 150…).
- Bakin.
