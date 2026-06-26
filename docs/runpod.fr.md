> 🇬🇧 [Read in English](runpod.md)

# Utiliser RunPod avec Hoshi2Star

Au lieu d'utiliser Ollama en local, vous pouvez louer un GPU cloud sur [RunPod](https://runpod.io) pour traduire plus vite avec des modèles plus puissants.

---

## Mise en place

1. Créez un pod sur RunPod (recommandé : RTX 4090 ou A40)
2. Configurez ces **Variables d'environnement** :
   ```
   OLLAMA_HOST   = 0.0.0.0
   OLLAMA_MODELS = /workspace/ollama-models
   ```
3. Exposez le port HTTP `11434`
4. Définissez cette **Commande de démarrage** :
   ```bash
   bash -c "mkdir -p /workspace/ollama-models && apt-get update && apt-get install -y zstd && curl -fsSL https://ollama.com/install.sh | sh && ollama serve & until ollama list > /dev/null 2>&1; do sleep 1; done && ollama pull qwen3:4b-instruct-2507-q8_0 && wait"
   ```
   > Pour utiliser un autre modèle, remplacez `qwen3:4b-instruct-2507-q8_0` par le vôtre — ex. `gemma4:26b-a4b-it-q4_K_M`. Les noms de modèles sont disponibles sur [ollama.com/library](https://ollama.com/library).
   >
   > `until ollama list` vérifie toutes les secondes que le serveur est prêt avant de télécharger le modèle — plus fiable qu'un `sleep 5` fixe.
5. Une fois le pod démarré, copiez l'URL proxy depuis RunPod → Connect :
   ```
   https://[POD_ID]-11434.proxy.runpod.net
   ```
6. Collez cette URL dans Hoshi2Star **Paramètres → URL Ollama** et cliquez sur **Tester**

---

## Tailles de lot recommandées

| GPU | VRAM | Taille de lot |
|---|---|---|
| RTX 4090 | 24 Go | 30–40 |
| A40 | 48 Go | 40–50 |
| A100 | 80 Go | 50–60 |

---

## Conseils

- **Arrêtez votre pod** après la traduction — RunPod facture à la minute.
- Les modèles stockés dans `/workspace` persistent entre les redémarrages (Volume disk), vous ne les téléchargez donc qu'une seule fois.
- Utilisez la variante `-instruct` du modèle (ex. `qwen3:4b-instruct-2507-q8_0`) — elle répond directement sans phase de réflexion, ce qui produit des traductions plus rapides et fiables.
