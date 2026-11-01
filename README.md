# echo-api

A tiny Rust ([axum](https://github.com/tokio-rs/axum)) REST API that echoes a path
parameter, built and pushed to a private registry by GitHub Actions and deployed
to a local Kubernetes cluster by Argo CD.

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

1. builds the Docker image and pushes `<REGISTRY>/<IMAGE_NAME>:<commit-sha>` and `:latest`;
2. commits the new tag into `k8s/kustomization.yaml`, which Argo CD syncs to the cluster.

Set these under **Settings → Secrets and variables → Actions**:

| Kind     | Name                | Example                                        |
| -------- | ------------------- | ---------------------------------------------- |
| Variable | `REGISTRY`          | `registry.example.com` (`docker.io` for Docker Hub) |
| Variable | `IMAGE_NAME`        | `echo-api` (`<dockerhub-user>/echo-api` for Docker Hub) |
| Secret   | `REGISTRY_USERNAME` | registry user                                  |
| Secret   | `REGISTRY_PASSWORD` | password or access token with push access      |

The registry must be reachable from GitHub-hosted runners.

## Local cluster — `cluster/`

k3s (single-node Kubernetes) in Docker. On startup it installs Argo CD
(`cluster/argocd-install.yaml`) and applies `argocd/application.yaml`, which
syncs `k8s/` from this repo's `master` branch into the `echo-api` namespace.

```bash
cd cluster
cp .env.example .env             # registry credentials k3s uses to pull the image
docker compose up -d
export KUBECONFIG=$PWD/kubeconfig.yaml
kubectl -n argocd get applications
```

- Argo CD UI: <http://localhost:8080> (user `admin`, password below)
- echo-api: <http://localhost:8081/echo/hello>

```bash
kubectl -n argocd get secret argocd-initial-admin-secret -o jsonpath='{.data.password}' | base64 -d
```

Argo CD polls the repo every ~2–3 minutes; press **Refresh** in the UI to sync sooner.
Tear down with `docker compose down -v`.

To use the Application on another cluster that already runs Argo CD:
`kubectl apply -f argocd/application.yaml`.
