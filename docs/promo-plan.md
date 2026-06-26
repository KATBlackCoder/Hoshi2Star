# Hoshi2Star — Plan de promotion réseaux sociaux

## Stratégie simplifiée — pour ceux qui ne sont pas à l'aise avec les réseaux

Le plan complet ci-dessous est volontairement exhaustif. Si tout ça semble trop, voici la version réduite à l'essentiel :

**Règle d'or : une étape à la fois, une plateforme à la fois.**

1. **Cette semaine — rien d'autre que le GIF.** Enregistre 30 secondes avec Kooha ou OBS. C'est le seul vrai bloqueur.
2. **Quand le GIF est prêt**, poster sur `r/fantranslation` uniquement. Le post est déjà rédigé (voir Phase 2), il suffit de copier-coller + ajouter le GIF.
3. **Attendre le résultat** avant de passer à la plateforme suivante.

> Les posts peuvent être rédigés par Claude — dis juste "rédige-moi le post pour r/fantranslation" et colle le résultat. Tu n'as qu'à poster.

---

## Diagnostic : pourquoi les premiers posts n'ont pas marché

Posts déjà tentés : f95zone, anime-sharing, r/RPGMaker — pas assez de traction.

| Plateforme | Problème probable |
|---|---|
| **f95zone** | Post sans GIF/vidéo — f95 ignore les posts texte. Ne pas reposter, éditer. |
| **anime-sharing** | Audience = téléchargeurs, pas traducteurs — mauvaise cible |
| **r/RPGMaker** | Bonne cible mais post trop technique ou sans démonstration visuelle |

---

## Phase 1 — Préparer les munitions (1-2 jours)

### 1.1 GIF de démo (PRIORITÉ ABSOLUE)

Sans GIF, aucun post ne décolle. 30 secondes qui montrent :
1. Ouvrir un projet Wolf/RPG Maker
2. Voir les segments japonais
3. Cliquer "Translate" → segments se remplissent en anglais
4. Score QA 100 qui apparaît

Outil recommandé : **Kooha** (Linux) ou OBS → convertir avec ffmpeg :

```bash
ffmpeg -i demo.mp4 -vf "fps=15,scale=800:-1:flags=lanczos" -loop 0 demo.gif
```

### 1.2 Deux versions du pitch

**Version courte (Twitter / titre Reddit) :**
> "I built a desktop tool that translates Japanese RPG games with a local LLM — no API key, no cloud, 27 seconds for a full game"

**Version longue (corps du post) :**
> Fan translating a Japanese RPG? This tool extracts all text, runs it through a local Ollama LLM, keeps a Translation Memory across projects, and reinjects everything back into the game files. Wolf RPG + RPG Maker MV/MZ supported. Free, open source, no API key needed.

### 1.3 Éditer le post f95zone existant (ne pas reposter)

f95zone punit les reposts. Éditer le post existant :
- Ajouter le GIF en premier (avant tout texte)
- Ajouter dans le corps : **"UPDATE: Now with LLM auto-translation (27s for a full game)"**
- Bumper le thread avec un commentaire "v0.4 released — Wolf RPG support added"

---

## Phase 2 — Nouveaux posts ciblés (semaine 1)

### Reddit — subreddits pas encore touchés

**Jour 1 — `r/fantranslation`** ← cible principale

```
Titre : "I made a free desktop tool for Japanese RPG fan translations
         — local LLM, no API key, Wolf RPG + RPG Maker supported"

[GIF en premier]

Been fan-translating Japanese RPGs and frustrated by the workflow.
Built this over 6 months: extracts all text, runs through local Ollama,
keeps TM across projects, reinjects into game files.

Wolf RPG Editor + RPG Maker MV/MZ. Free, open source.

Looking for feedback from actual translators.

[lien GitHub]
```

**Jour 3 — `r/LocalLLaMA`** — angle LLM local, pas traduction
```
Titre : "Built a CAT editor that uses local Ollama to translate
         Japanese RPG games — full pipeline with TM and QA"
```
Montrer les stats : modèle utilisé, tokens/s, qualité vs GPT-4.

**Jour 5 — `r/JRPG`** — angle utilisateur final
> "You can now play untranslated Japanese RPGs"

**Jour 7 — `r/gamedev`** — angle technique : architecture Tauri/Rust, pipeline LLM.

### Discord — action immédiate

Channels `#tools` ou `#resources` à cibler :
- RPG Maker Web Discord (officiel)
- Wolf RPG Editor Discord (Discord Discovery)
- Tauri Discord — channel `#showcase`
- Rechercher : "fan translation discord", "JRPG translation discord"

### Twitter/X — thread en 5 tweets

1. GIF + phrase choc
2. "How it works" — 3 bullet points
3. Screenshot avant/après traduction
4. Stack technique (pour les devs)
5. Lien GitHub + call to action

Hashtags : `#gamedev` `#indiedev` `#JRPG` `#fantranslation` `#Rust` `#OpenSource`

---

## Phase 3 — Canaux spécialisés (semaine 2)

### Hacker News — Show HN

Format strict, fort trafic :
```
Show HN: Hoshi2Star – local-LLM CAT editor for Japanese RPG fan translations
```
- Poster lundi ou mardi matin 9h EST
- Répondre à TOUS les commentaires dans les 2h

### GitHub — Être référencé

Soumettre une PR à :
- `awesome-tauri`
- `awesome-rust` — section "applications"
- Commenter dans les issues des projets liés (Wolf RPG tools, RPG Maker tools) avec un lien naturel

### YouTube (optionnel mais fort)

Vidéo de 5 minutes :
1. Problème (30s) : "translating Japanese RPGs is painful"
2. Démo live (3 min) : ouvrir → traduire → exporter → jouer
3. Installation (1 min)
4. Call to action

Titre SEO : *"Translate Japanese RPG Games with AI — Free Tool (Wolf RPG + RPG Maker)"*

---

## Checklist

### Version simplifiée (recommandée si les réseaux te semblent trop)

```
Étape 1 (cette semaine) :
[ ] Créer le GIF de démo — 30 secondes, Kooha ou OBS (BLOQUE TOUT LE RESTE)

Étape 2 (quand le GIF est prêt) :
[ ] Poster r/fantranslation — copier-coller le post de la Phase 2, ajouter le GIF
[ ] Attendre 48h et observer le résultat

Étape 3 (si r/fantranslation a eu de la traction) :
[ ] Éditer le post f95zone existant avec le GIF + bump
[ ] Poster dans 1 Discord ciblé (RPG Maker Web ou Wolf RPG Editor)

Étape 4 (semaine suivante) :
[ ] Choisir une seule plateforme parmi : r/LocalLLaMA, Twitter, Show HN
```

### Version complète (quand tu te sens prêt)

```
Semaine 1 :
[ ] 1. Créer le GIF de démo (bloque tout le reste)
[ ] 2. Éditer le post f95zone existant avec le GIF + bump
[ ] 3. Poster r/fantranslation
[ ] 4. Poster dans 2-3 Discords ciblés
[ ] 5. Thread Twitter

Semaine 2 :
[ ] 6. r/LocalLLaMA, r/JRPG, r/gamedev
[ ] 7. Show HN
[ ] 8. PRs awesome-tauri / awesome-rust
```
