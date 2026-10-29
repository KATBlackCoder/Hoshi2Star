# Grille de segments — mesure dans l'app réelle (2026-10-29, Claude)

**But :** vérifier sur la machine de l'utilisateur, dans la vraie app Tauri, les gains de G1 mesurés en cloud (`docs/benchmarks/2026-10-29-grid-g1.md`, branche `perf/grid-scroll`), puis trouver la cause de la lenteur restante.

**Conditions**
- App de dev (`pnpm tauri dev`) pilotée par le pont MCP (`127.0.0.1:9223`).
- Machine : Wayland, NVIDIA GTX 1660 SUPER (pilote 615.78.08), `webkit2gtk-4.1` 2.54.2.
- Vrai fichier : トロイメライ `CommonEvent.dat`, 12 423 segments.
- Fenêtre de 1600×900 CSS px, zone de défilement de 562 px de haut.
- Défilement scripté : `scrollTop` avancé de 120 px à chaque `requestAnimationFrame`.
- Durées de frame relevées par `requestAnimationFrame`. Lecture seule : rien n'a été écrit en base.
- Pour passer de `main` à G1, j'ai seulement changé de branche (Vite recharge la page) ; Rust n'a pas été recompilé.

**Limite :** build de dev (React en développement + `StrictMode`). Les durées absolues sont plus élevées qu'en build de production, mais les comparaisons restent valables.

## 1. G1 dans l'app réelle (avec `pnpm tauri:linux`)

| Mesure | `main` | G1 (`59bd545`) |
|---|---|---|
| Défilement, page fraîchement chargée | 655 px/s, frame p50 150 ms | 732 px/s (+12 %), p50 137 ms |
| Défilement, lignes déjà mesurées | 1 039 px/s, p50 97 ms | 1 366 px/s (+31 %), p50 94 ms |
| Frames avec zone blanche, sauts de 600 px | **0 / 120** | **4 / 120**, jusqu'à 277 px |
| Frames avec zone blanche, sauts de 3 000 px | **0 / 60** | **12 / 60**, jusqu'à 553 px (presque l'écran entier) |

- Une première mesure G1 (915 px/s) a été écartée : une mesure précédente tournait encore en parallèle.
- **G1 crée des zones blanches dans l'app réelle**, alors que le banc cloud (Chromium et WebKit de Playwright) en mesurait 0. Il ne faut pas le fusionner tel quel.
- Dans tous les cas, la grille tourne à environ **7 à 10 images par seconde**, avec ou sans G1. G1 ne règle pas la plainte de l'utilisateur.

## 2. Isoler la cause (sur `main`, avec `pnpm tauri:linux`)

Toutes les variantes ont été mesurées après un passage de chauffe (lignes déjà mesurées), sur 150 frames, en injectant du CSS à la volée :

| Variante | Frame p50 |
|---|---|
| Référence | 95 ms |
| Sans `text-wrap: pretty` ni `optimizeLegibility` | 96 ms |
| Zones de saisie (`textarea`) masquées | 93 ms |
| Lignes invisibles (`visibility: hidden`) | 94 ms |
| **Gestionnaire de défilement (rendu React)** | **17 ms** (p95 21 ms) |
| **Bloc de test sans React** (3 000 `<p>`, 900×560) | **78 ms** (p95 267 ms) |
| Fenêtre au repos | 16 ms |

**Conclusion :** même un bloc sans React défile à environ 13 images par seconde. Le coût vient du rendu de la fenêtre par WebKitGTK, pas du code de la grille. React ne pèse qu'environ 17 ms par frame en build de dev.

## 3. Sans `WEBKIT_DISABLE_DMABUF_RENDERER=1`

L'app a été relancée sur `main` avec seulement `__NV_DISABLE_EXPLICIT_SYNC=1`, sans `WEBKIT_DISABLE_DMABUF_RENDERER=1`, alors que le script `tauri:linux` de `package.json` impose les deux :

| Mesure | Avec la variable | Sans |
|---|---|---|
| Grille, frame p50 / p95 | 95 / 98 ms (10,5 images/s) | **19 / 22 ms (52,6 images/s)** |
| Bloc de test sans React | 78 ms | **17 ms** |
| Gestionnaire de défilement | 17 ms | 15 ms |

**La lenteur ressentie vient surtout de `WEBKIT_DISABLE_DMABUF_RENDERER=1`**, qui désactive le rendu accéléré de WebKitGTK. Le script `tauri:linux` l'applique alors que la session est en Wayland, et `CONTEXT.md` le réserve pourtant à X11.

**Non vérifié :**
- que la fenêtre s'affiche correctement à l'écran sans la variable (la capture du pont lit le contenu de la page, pas l'écran) : à confirmer par l'utilisateur ;
- les zones blanches de G1 avec le rendu accéléré ;
- l'app installée (build de production). Elle ne définit pas la variable (rien dans `src-tauri/`, rien dans la configuration du shell), donc elle n'est pas concernée, sauf si elle est lancée par ce script.

## Suites proposées

1. Retirer `WEBKIT_DISABLE_DMABUF_RENDERER=1` de `tauri:linux` (Wayland), et garder une variante X11 à part. À faire après confirmation visuelle par l'utilisateur.
2. Ne pas fusionner G1 en l'état : refaire la mesure des zones blanches avec le rendu accéléré, et corriger le contrôle hybride si elles persistent.
3. Les lots P2 à P5 de l'audit restent utiles, mais leur gain ressenti sera bien plus faible que celui du point 1.

## 4. G1 remesuré avec le rendu accéléré (après le correctif de `tauri:linux`)

App lancée avec le `pnpm tauri:linux` corrigé (`main` @ `5f2d9ad`), même fichier, même méthode. Chaque mesure enchaîne un passage de chauffe, puis deux passages mesurés de 300 frames. Les deux mesures `main` encadrent la mesure G1 :

| Mesure | `main` (1) | G1 (`59bd545`) | `main` (2) |
|---|---|---|---|
| Frame p50 (passages A / B) | 21 / 20 ms | 21 / 21 ms | 22 / 22 ms |
| Frame p95 | 24 / 24 ms | 25 / 24 ms | 26 / 27 ms |
| Frames > 33 ms (sur 300) | 1 / 1 | 0 / 0 | 1 / 1 |
| Gestionnaire de défilement p50 | 17 / 16 ms | **0 ms** | 18 / 18 ms |
| Zones blanches, sauts de 600 px | 0 / 150 | 0 / 150 | — |
| Zones blanches, sauts de 3 000 px | 0 / 100 | 0 / 100 | — |

- **Pas de gain mesurable.** G1 sort le rendu React de l'événement de défilement (0 ms au lieu de 17 ms), mais le même travail est fait juste après : la durée des frames ne change pas, l'écart reste dans le bruit des mesures.
- Avec le rendu accéléré, G1 ne crée plus de zones blanches. Celles de la section 1 venaient de frames de 95 ms.
- Conclusion : **G1 est abandonné**. Il ajoute de la complexité (`flushSync` conditionnel, plage rendue suivie après chaque commit) sans gain.
- Le reste du coût est le rendu React d'environ 17 ms par frame en build de dev (`StrictMode`, React en développement). En build de production, ce coût devrait être nettement plus faible : non mesuré.
