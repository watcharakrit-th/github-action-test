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

| Method | Path              | Response               |
| ------ | ----------------- | ---------------------- |
| GET    | `/echo/{message}` | `{"echo":"<message>"}` |
| GET    | `/healthz`        | `ok`                   |

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

| File               | What                                                                 |
| ------------------ | -------------------------------------------------------------------- |
| `echo-api.yaml`    | the Rust API (2 replicas) and its cluster-internal Service           |
| `nginx.yaml`, `nginx.conf` | reverse proxy to echo-api; NodePort 30081                    |
| `cloudflared.yaml` | Cloudflare quick tunnel to nginx (random `*.trycloudflare.com` URL)  |

## Local cluster — `cluster/`

k3s (single-node Kubernetes) in Docker. On startup it installs Argo CD
(`cluster/argocd-install.yaml`) and applies `argocd/application.yaml`, which
syncs `k8s/` from this repo's `master` branch into the `echo-api` namespace.

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
