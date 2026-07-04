# Publier Hoshi2Star sur itch.io — guide complet

> Objectif : utiliser **itch.io comme vitrine et point de téléchargement**, tout en
> gardant **GitHub Releases comme source de vérité** (c'est là que pointe l'updater
> intégré, cf. ADR-007). Ce document explique le pourquoi, la mise en place pas à pas,
> l'automatisation CI, et les pièges à connaître.

---

## 1. Pourquoi itch.io convient

| Raison | Détail |
|--------|--------|
| **itch accepte les outils** | À la création d'un projet, le champ *Classification* propose **Tools** (pas seulement *Games*). Un éditeur CAT + assistant LLM de traduction y a toute sa place. |
| **Le bon public** | La communauté **RPG Maker / Wolf RPG / modding / fan-translation** est très concentrée sur itch — bien plus qu'une page GitHub Releases. Meilleure découvrabilité auprès des utilisateurs cibles. |
| **Aucun risque légal** | On distribue un **logiciel** (licence MIT), pas des jeux traduits ni du contenu copyrighté. Un outil ne tombe pas sous le DMCA comme le ferait un jeu redistribué. Le tool n'étant pas du contenu adulte, **pas besoin du flag « adult »** même si les utilisateurs traduisent des jeux F95. |
| **Fonctions de vitrine** | Page projet avec cover + screenshots, **devlogs** pour annoncer les updates, bouton download clair, option **pay-what-you-want / dons** (compatible MIT). |

**Positionnement recommandé** :
- **GitHub** = cœur technique + source de l'auto-updater.
- **itch** = vitrine, découvrabilité, téléchargement initial, dons.

---

## 2. Prérequis

1. Un compte itch.io (créateur).
2. Les builds produites par la CI (`release.yml`) : `.msi` (Windows) et `.AppImage`
   (Linux) au minimum — ce sont les seules cibles auto-updatables, cf. §6.
3. Pour l'automatisation : l'outil **butler** (CLI officiel d'itch) + une **API key**.

---

## 3. Créer la page projet (une seule fois)

Sur `itch.io/dashboard` → **Create new project**.

| Champ | Valeur conseillée |
|-------|-------------------|
| **Title** | Hoshi2Star |
| **Project URL** | `katblackcoder.itch.io/hoshi2star` |
| **Short description / tagline** | *CAT editor + LLM translator for Japanese RPG fan games (星 → ★)* |
| **Classification** | **Tools** |
| **Kind of project** | **Downloadable** (pas HTML/web) |
| **Release status** | Released (ou In development si tu préfères) |
| **Pricing** | *No payments* ou *Pay what you want* (dons possibles, min à 0) |
| **Platforms** | Windows + Linux (cases à cocher sur chaque upload) |
| **Cover image** | 630×500 px recommandé (min 315×250) |
| **Screenshots** | Réutilise `docs/screenshots/` (avant/après JP→EN, interface 3 panneaux) |
| **Visibility** | *Draft* → *Restricted* (test) → *Public* |

**Métadonnées utiles** : tags `translation`, `rpg-maker`, `tool`, `localization`,
`wolf-rpg`, `japanese`. Lien vers le dépôt GitHub + le CHANGELOG dans la description.

> Astuce : coche **« Hide this project from browse pages until… »** tant que tu testes,
> puis rends-le public quand la page est prête.

---

## 4. Uploader les builds

Deux méthodes. **butler est fortement recommandé** (versionné, diffs, updates via l'app itch).

### 4.a — Manuel (web)
Onglet *Uploads* → *Upload files* → glisse le `.msi` et le `.AppImage`. Coche la
plateforme correspondante sur chaque fichier. L'AppImage fait ~88 Mo, bien en dessous
de la limite web (1 Go/fichier), donc l'upload manuel reste possible.

### 4.b — butler (recommandé)

**Installation + login (une fois) :**
```bash
# Linux
curl -L -o butler.zip https://broth.itch.zone/butler/linux-amd64/LATEST/archive/default
unzip butler.zip && chmod +x butler
./butler login          # ouvre le navigateur pour authentifier
```

**Pousser une build** (le *channel* encode la plateforme — itch détecte `windows`/`linux`) :
```bash
# Depuis le dossier où sont les artefacts, VERSION = ex. 0.4.5
./butler push hoshi2star_0.4.5_amd64.AppImage  katblackcoder/hoshi2star:linux   --userversion 0.4.5
./butler push hoshi2star_0.4.5_x64_en-US.msi   katblackcoder/hoshi2star:windows --userversion 0.4.5
```

- Le nom du **channel** (`linux`, `windows`) suffit à itch pour taguer la plateforme.
- `--userversion` affiche la version proprement sur la page et dans l'app itch.
- butler ne ré-uploade que les **diffs** (wharf) → rapide sur les releases suivantes.

---

## 5. Automatiser dans la CI (`release.yml`)

L'idée : après le build tauri, pousser l'artefact de **chaque** runner vers son channel.
On installe butler sur le runner et on lit une API key depuis les secrets.

### 5.a — Secret à poser (une fois)
`itch.io/user/settings/api-keys` → génère une clé → ajoute-la en secret GitHub :
```bash
gh secret set BUTLER_API_KEY
```
butler lit automatiquement la variable d'env `BUTLER_API_KEY` (pas besoin de `login`).

### 5.b — Step à ajouter au job `publish-tauri` (après `tauri-action`)

> ⚠️ À adapter : le chemin exact des artefacts dépend de la sortie de tauri-action.
> `tauri-action` expose `steps.<id>.outputs.artifactPaths` (JSON). Le plus robuste est
> de donner un `id:` au step tauri-action et de pousser depuis ces chemins, ou de
> cibler le dossier `src-tauri/target/release/bundle/`.

```yaml
      - name: Publish to itch.io
        if: startsWith(github.ref, 'refs/tags/v')
        shell: bash
        env:
          BUTLER_API_KEY: ${{ secrets.BUTLER_API_KEY }}
        run: |
          VERSION="${GITHUB_REF_NAME#v}"

          # Installer butler (Linux runner ; adapter windows-* → butler/windows-amd64)
          if [ "${{ matrix.platform }}" = "ubuntu-22.04" ]; then
            curl -L -o butler.zip https://broth.itch.zone/butler/linux-amd64/LATEST/archive/default
            unzip -q butler.zip && chmod +x butler
            FILE=$(ls src-tauri/target/release/bundle/appimage/*.AppImage | head -1)
            ./butler push "$FILE" katblackcoder/hoshi2star:linux --userversion "$VERSION"
          else
            curl -L -o butler.zip https://broth.itch.zone/butler/windows-amd64/LATEST/archive/default
            7z x butler.zip -obutler-bin >/dev/null
            FILE=$(ls src-tauri/target/release/bundle/msi/*.msi | head -1)
            ./butler-bin/butler.exe push "$FILE" katblackcoder/hoshi2star:windows --userversion "$VERSION"
          fi
```

**Alternative plus simple** : une Action toute faite (ex. `KikimoraGames/itch-publish`
ou `manleydev/butler-publish`) qui encapsule butler. Le principe reste identique
(API key en secret + channel + fichier). L'approche butler brute ci-dessus évite une
dépendance d'Action supplémentaire et reste transparente.

> Note : garder ce push **derrière `if: startsWith(github.ref, 'refs/tags/v')`** pour
> ne publier sur itch que sur un vrai tag de release, jamais sur un `workflow_dispatch`
> de test.

---

## 6. Interaction avec l'auto-updater (important)

L'updater intégré (ADR-007) interroge
`github.com/KATBlackCoder/Hoshi2Star/releases/latest/download/latest.json`.
**Il pointe vers GitHub, quelle que soit la provenance du téléchargement.**

Conséquences :
- Un utilisateur qui télécharge depuis **itch** se mettra quand même à jour **depuis
  GitHub** via le updater intégré. Ça fonctionne — mais ce n'est pas l'app itch qui gère
  la mise à jour.
- **Ne pas mélanger** les deux mécanismes de mise à jour. Deux options propres :
  1. **(Recommandé)** GitHub reste la source des updates ; itch = vitrine + download
     initial. Simple, cohérent avec l'updater déjà en place.
  2. Tout miser sur l'**app itch** (updates via channels butler) — mais alors l'updater
     intégré fait doublon. À éviter tant que l'updater GitHub est actif.
- **Cibles updatables** : seules **Windows (NSIS/msi)** et **Linux AppImage** s'auto-
  mettent à jour (`updater_supported()` = `cfg!(windows) || APPIMAGE`). Sur itch, publie
  donc en priorité le `.msi` et l'`.AppImage`. Un `.deb`/`.rpm` sur itch serait un cul-de-
  sac pour l'updater — à ne pas mettre en avant.

---

## 7. Paiements / payouts (ToS itch.io — lu le 2026-07-03)

> Analyse des Terms of Service itch.io (version du 2023-04-15) appliquée au cas
> Hoshi2Star : outil MIT, gratuit sur GitHub → paiements = **dons**. Verdict :
> **rien de bloquant**, ToS standards, mais 3 points concrets à connaître.

### ⚠️ Le piège : earnings non réclamés (§10 des ToS)
Les revenus **non retirés depuis plus de 12 mois** subissent une ponction de
**10 % par mois** jusqu'à zéro (≈ tout disparaît vers 22 mois). itch n'est pas
une banque. **Règle : faire un payout au moins une fois par an** — mettre un
rappel annuel.

### Configuration recommandée
1. **Pricing : « Pay what you want », minimum 0 $** — cohérent avec un outil MIT
   gratuit sur GitHub. Les paiements deviennent des dons → quasi aucun cas de
   refund ni d'attente de support « client payant ».
2. **Payout mode : « Collected by itch.io, paid later »** (itch = *merchant of
   record*) — itch collecte et gère TVA/taxes de vente à ta place. Éviter le mode
   « direct to PayPal » : le Publisher y est responsable des remboursements et de
   la TVA lui-même (§9 des ToS).
3. **Revenue share** : configurable par le Publisher (défaut 10 % pour itch,
   modifiable — laisser ~10 % est le geste standard).
4. **W-8BEN** : à remplir dès l'activation des payouts (formulaire en ligne,
   Publisher non-US). Sans lui, la retenue fiscale US est maximale (30 %).
   Les revenus restent à déclarer dans le pays de résidence.

### Points vérifiés, non bloquants
- **§4 (licences)** : itch reçoit une licence promo (screenshots, distribution sur
  la plateforme) et les acheteurs une licence perpétuelle sur le téléchargement —
  non-événement pour un logiciel déjà sous MIT ; la propriété reste au Publisher.
- **§7 (processeurs)** : respecter les AUP PayPal/Stripe/Payoneer — aucun risque
  pour un outil de traduction (ces politiques visent contenu adulte, armes, etc.).
- **§9 (refunds)** : remboursement si le produit ne fonctionne pas ; avec un
  minimum à 0 $, quasi aucun cas réel.
- **§14/15** : droit californien + class action waiver — standard plateforme US.
- **§6 (DMCA)** : non applicable — l'outil ne distribue aucun contenu de jeu.

---

## 8. Checklist par release

1. La release GitHub `vX.Y.Z` est **publiée** (pas draft) — l'updater et itch en dépendent.
2. `butler push` du `.AppImage` (channel `linux`) et du `.msi` (channel `windows`) avec
   `--userversion X.Y.Z` — manuel ou via CI (§5).
3. Vérifier la page itch : la nouvelle version s'affiche, les deux plateformes sont là.
4. Rédiger un **devlog** itch (peut reprendre le post d'annonce / le CHANGELOG).
5. Laisser la version précédente accessible si besoin (itch garde l'historique des builds).

---

## 9. Résumé

- **Oui, publie sur itch** : c'est fait pour les outils, c'est là qu'est le public
  RPG Maker/Wolf, et c'est légalement sûr (outil MIT, pas de contenu redistribué).
- **GitHub reste le cœur** (updater + source de vérité) ; **itch est la vitrine**.
- **Le seul vrai travail** = automatiser `butler push` dans `release.yml` (§5) pour ne
  pas uploader à la main à chaque version.
- **Ne pas** faire dépendre l'auto-update de itch : l'updater intégré pointe sur GitHub,
  garde-le comme unique mécanisme d'update.
- **Paiements = dons** (pay-what-you-want min 0 $, itch merchant of record, W-8BEN) —
  et **payout au moins 1×/an** sous peine de ponction 10 %/mois après 12 mois (§7).

---
*Guide rédigé pour Hoshi2Star — voir aussi ADR-007 (auto-updater) et `docs/promo-plan.md`.*
