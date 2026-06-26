> 🇫🇷 [Lire en français](runpod.fr.md)

# Using RunPod with Hoshi2Star

Instead of running Ollama locally, you can use a cloud GPU on [RunPod](https://runpod.io) for faster translation with larger models.

---

## Setup

1. Create a pod on RunPod (recommended: RTX 4090 or A40)
2. Set these **Environment Variables**:
   ```
   OLLAMA_HOST   = 0.0.0.0
   OLLAMA_MODELS = /workspace/ollama-models
   ```
3. Expose HTTP port `11434`
4. Set this **Start Command** (replace the model name if needed):
   ```bash
   bash -c "mkdir -p /workspace/ollama-models && apt-get update && apt-get install -y zstd && curl -fsSL https://ollama.com/install.sh | sh && ollama serve & until ollama list > /dev/null 2>&1; do sleep 1; done && ollama pull qwen3:4b-instruct-2507-q8_0 && wait"
   ```
   > `until ollama list` polls every second until the server is ready before pulling the model — more reliable than a fixed `sleep 5`.
5. Once the pod is running, copy the proxy URL from RunPod → Connect:
   ```
   https://[POD_ID]-11434.proxy.runpod.net
   ```
6. Paste this URL in Hoshi2Star **Settings → Ollama URL** and click **Test**

---

## Recommended Batch Sizes

| GPU | VRAM | Batch size |
|---|---|---|
| RTX 4090 | 24 GB | 30–40 |
| A40 | 48 GB | 40–50 |
| A100 | 80 GB | 50–60 |

---

## Tips

- **Stop your pod** after translation — RunPod charges per minute.
- Models stored in `/workspace` persist across restarts (Volume disk), so you only download them once.
- Use the `-instruct` model variant (e.g. `qwen3:4b-instruct-2507-q8_0`) — it responds directly without a thinking phase, which produces faster and more reliable translations.
