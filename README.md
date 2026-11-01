# echo-api

A tiny Rust ([axum](https://github.com/tokio-rs/axum)) REST API that echoes a path
parameter. GitHub Actions tests it and pushes the image to Docker Hub
(`watcharakritth/mini-rust`); Argo CD deploys it to a local Kubernetes cluster
behind nginx, and a free Cloudflare Tunnel exposes it publicly.

```
internet ──▶ Cloudflare quick tunnel ──▶ cloudflared ──▶ nginx ──▶ echo-api
                       localhost:8081 ──────────────────────▲
```

## API

| Method | Path              | Response                  |
| ------ | ----------------- | ------------------------- |
| GET    | `/`               | usage hint (plain text)   |
| GET    | `/echo/{message}` | `{"echo":"<message>"}`    |
| GET    | `/version`        | `{"version":"<git sha>"}` |
| GET    | `/healthz`        | `ok`                      |

```bash
cargo run                         # listens on :8080 (override with PORT)
curl localhost:8080/echo/hello    # {"echo":"hello"}
cargo test
```

## CI/CD — `.github/workflows/ci.yml`

Every push runs `cargo fmt --check`, `cargo clippy` and `cargo test`. On `master` it then:

1. builds the image and pushes `docker.io/watcharakritth/mini-rust:<commit-sha>` and `:latest`;
2. commits the new tag into `k8s/kustomization.yaml`, which Argo CD syncs to the cluster.

Required: a repository secret **`DOCKERHUB_TOKEN`** (Docker Hub access token with
Read & Write scope) under **Settings → Secrets and variables → Actions**.

## Kubernetes — `k8s/`

| File                       | What                                                                |
| -------------------------- | ------------------------------------------------------------------- |
| `echo-api.yaml`            | the Rust API as a blue-green `Rollout`, plus active/preview Services |
| `nginx.yaml`, `nginx.conf` | reverse proxy to echo-api; NodePort 30081                           |
| `cloudflared.yaml`         | Cloudflare quick tunnel to nginx (random `*.trycloudflare.com` URL) |

## Blue-green releases

`echo-api` is an [Argo Rollouts](https://argoproj.github.io/rollouts/) `Rollout`. When CI
bumps the image, the new version ("green") starts next to the current one ("blue"):
`echo-api-preview` points at green, while `echo-api` (behind nginx and the tunnel) stays on
blue. The Argo CD app shows **Suspended** until someone promotes.

```bash
curl localhost:8081/version                                     # blue: what users get
kubectl -n echo-api port-forward svc/echo-api-preview 8082:80   # in a second terminal
curl localhost:8082/version                                     # green: preview only
```

- **Promote:** Argo CD UI → `echo-api` app → click the Rollout → ⋮ → **Resume**. Traffic
  switches at once; blue is scaled down 30 s later.
- **Cancel before promoting:** ⋮ → **Abort**, then `git revert` CI's deploy commit so git
  matches what runs.
- **Roll back after promoting:** `git revert` CI's deploy commit; that is a new release
  (old image as green) and waits for **Resume** again.

## Local cluster — `cluster/`

k3s (single-node Kubernetes) in Docker. On startup it installs Argo CD
(`cluster/argocd-install.yaml`) and Argo Rollouts (`cluster/argo-rollouts-install.yaml`),
and applies `argocd/application.yaml`, which syncs `k8s/` from this repo's `master`
branch into the `echo-api` namespace.

```bash
cd cluster
cp .env.example .env             # Docker Hub token k3s uses to pull images
docker compose up -d
export KUBECONFIG=$PWD/kubeconfig.yaml
kubectl -n argocd get applications
```

- Argo CD UI: <http://localhost:8080> — user `admin`, password:
  `kubectl -n argocd get secret argocd-initial-admin-secret -o jsonpath='{.data.password}' | base64 -d`
- echo-api via nginx: <http://localhost:8081/echo/hello>
- Public URL (changes whenever the cloudflared pod restarts):
  `kubectl -n echo-api logs deploy/cloudflared | grep -o 'https://[a-z0-9-]*\.trycloudflare\.com'`

Argo CD polls the repo every ~2–3 minutes; press **Refresh** in the UI to sync sooner.
Tear down with `docker compose down -v`.

To use the Application on another cluster that already runs Argo CD:
`kubectl apply -f argocd/application.yaml`.
