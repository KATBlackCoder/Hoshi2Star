# Hoshi2Star — BACKLOG

> Idées futures non planifiées. Rien ici ne fait partie d'une phase active.
> Évaluer au cas par cas selon la traction utilisateurs et le retour beta.

---

## TM Panel — améliorations rapides

> Avant le TM Browser complet : couvre 80% du besoin en 20% de l'effort.

- [ ] Indicateur de confiance visuel par suggestion (barre ou badge `95%`, couleur selon seuil)
- [ ] Distinction visuelle `auto` (LLM) vs `manual` (validé humain) sur chaque suggestion
- [ ] Bouton "Supprimer cette entrée TM" depuis le panneau latéral (avec confirmation)
- [ ] Bouton "Corriger" — éditer la traduction d'une entrée TM sans quitter l'éditeur
- [ ] Tooltip sur chaque suggestion : projet d'origine + date + moteur

---

## TM Browser — page de gestion Translation Memory

> Interface dédiée pour consulter, corriger et gérer toutes les entrées TM.

- [ ] Command `list_tm_entries(filters, page, page_size)` — liste paginée avec filtres (moteur, langue, projet, confiance, date, statut)
- [ ] Command `update_tm_entry(id, target_text)`
- [ ] Command `delete_tm_entries(ids: Vec<String>)` — suppression unitaire ou en masse
- [ ] Command `create_tm_entry(source, target, engine, lang_pair)`
- [ ] `src/features/tm/TMBrowser.tsx` — tableau paginé (source, target, moteur, langue, confiance, projet, date)
- [ ] `src/features/tm/TMFilters.tsx` — barre de filtres
- [ ] `src/features/tm/TMEntryDialog.tsx` — dialog create/edit
- [ ] Bulk select + bulk delete avec confirmation
- [ ] Stats en en-tête : `X entrées · Y projets · Z langues`
- [ ] Accessible depuis un onglet dédié dans la navigation principale

---

## Pipeline LLM — robustesse avancée

*Inspiré de l'analyse DazedMTLTool (2026-06-14)*

- [ ] QA contenu post-traduction : détecter traduction vide, trop courte vs source, "runaway", répétition de caractères ; retry avec note de correction dans le message user
- [ ] Historique glissant (N derniers segments traduits) injecté comme contexte de cohérence
- [ ] Mode "Estimate" : calcul tokens/coût avant de lancer une traduction réelle
- [ ] Cache de traduction disque (hash payload → résultat) pour dédup cross-run avant validation humaine
- [ ] Prompt caching Claude : séparer prompt statique (glossaire global + règles, cache ephemeral 1h) du contexte dynamique
- [ ] Batch API Anthropic (50% moins cher) : pipeline 2 passes collect/consume — complexité élevée, gros projets uniquement
- [ ] Adaptive rate limiter par provider (lecture headers `x-ratelimit-*`)
- [ ] `OpenAIProvider` (clé user fournie)
- [ ] `DeepSeekProvider`
- [ ] Passe 2 LLM : review (consistency sur fenêtre de 10 segments)

---

## Glossaire — enrichissement

- [ ] Champs optionnels par terme/personnage : genre, rôle, registre de discours (ex: "flustered", "cold and terse") injectés dans le prompt
- [ ] Matching contextuel : n'injecter que les termes réellement présents dans le batch courant (word-boundary aware kanji/kana)

---

## Engine MV/MZ — couverture avancée

- [ ] Toggles granulaires par code d'événement (101/102/108/111/122/320/324/355/356/357/401/405/408/657) configurables par projet
- [ ] Speaker detection heuristique : scan maps + scoring de patterns (`\n<Name>`, `【Name】`, `Name「...`, code 101)
- [ ] QA largeur de ligne "visible length" ignorant les codes couleur `\c[n]`

---

## Features différenciantes

- [ ] Overlay playtest "click-to-segment" : plugin injecté dans le jeu (NW.js) qui, via un serveur local exposé par Tauri, sélectionne le segment dans SegmentGrid pendant le playtest
- [ ] Prompt par projet en surcouche : règles custom (ton, avertissements, terminologie) fusionnées avec le prompt de base
- [ ] Traduction d'images (OCR + inpainting + ré-encryption `.rpgmvp`) — complexité élevée
- [ ] Export format `.po/.pot` (interopérabilité avec d'autres outils CAT)
- [ ] Cloud TM partagé opt-in (anonymisé) — contribution communautaire
- [ ] Plugin VSCode pour éditer les segments directement dans l'IDE
- [ ] App mobile companion (lecture seule du projet, validation segments)
- [ ] Support RPG Maker 2000/2003 (vgperson workflow, niche)
