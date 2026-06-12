# Persistance incrémentale par batch + cooldown à grain fin — DONE

Implémenté selon `/home/blackat/.claude/plans/bright-tumbling-fountain.md`.

- [x] `progress.rs` : `CoolingPayload { remaining_secs: u64 }` (camelCase)
- [x] `pipeline.rs` :
  - `CooldownState::new(threshold_secs, duration_secs)` + `maybe_rest(&mut self, handle).await`
  - `persist_batch_results(db, &batch_results).await` (UPDATE par batch, erreurs loguées)
  - `run_inner` : nouveaux paramètres `app_handle: Option<&AppHandle>`,
    `cooldown: Option<&mut CooldownState>` ; persistance + emit progress/warning
    + cooldown await tous dans la boucle `for batch_ids in &batches` (séquentiel,
    pas de canal/`join!` — le cooldown suspend réellement la traduction)
  - `run` devient un wrapper fin sur `run_inner`
  - 10 tests mis à jour (`None, None,` ajoutés)
- [x] `commands/translate.rs` :
  - `translate_segments` : `pipeline::run(..., None)`, suppression de la boucle UPDATE, `Ok(_)`
  - `translate_all_segments` : `CooldownState::new(...)` créé une fois avant la
    boucle fichiers, `pipeline::run(..., Some(&mut cooldown))`, suppression de
    la boucle UPDATE et du bloc cooldown de fin de fichier

## Vérification

- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml`
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` → clean
- [x] `cargo test --manifest-path src-tauri/Cargo.toml` → 302 passed, 0 failed
- [ ] Test manuel "Translate All" sur un gros fichier avec threshold/cooldown
      courts (cooldown mid-fichier + persistance incrémentale) — à faire par
      l'utilisateur

## Hors scope (non traité)

- Écriture TM après traduction LLM fraîche (TM reste lecture seule côté pipeline)
- Stats manifest (mises à jour fin de fichier/traduction, pré-existant)
- Pause/resume manuel (CancellationToken + boutons UI)
