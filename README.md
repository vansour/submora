# submora

GitHub: https://github.com/vansour/submora

`submora` 是一个面向多用户订阅聚合场景的 Rust 项目。后端使用 Axum `0.8.8`，前端使用 `Vue 3 + Vite + TypeScript`，提供单页管理台、管理员账户管理、订阅组维护，以及 `GET /{username}` 公共聚合路由。

## 架构

- 前端：`web/`
- 后端：`backend/`
- 共享协议与校验：`backend/src/shared.rs`、`backend/src/core.rs`
- 仓库说明：`README.md`

目录树说明：

```text
.
├── web/               # Vue 3 + Vite 管理台
├── backend/           # Axum 服务、公共聚合路由、管理 API
├── scripts/           # 辅助脚本、测试运行脚本
├── Dockerfile         # 生产镜像构建入口
└── compose.yml        # 默认部署入口
```

## 当前能力

- 管理员登录、登出、恢复会话
- 管理员账户更新，成功后强制重新登录
- 订阅组创建、删除、排序
- 逐行链接编辑、新增、删除、拖拽排序、本地格式校验
- 保存后使用服务端归一化结果回填
- 复制公共聚合入口链接
- diagnostics 与 cache 面板展示
- 手动刷新缓存、清空缓存
- 公共聚合入口 `GET /{username}`，返回 `text/plain`

## Docker 部署

前提：

- Docker
- Docker Compose

启动：

```bash
docker compose up -d --build
```

停止：

```bash
docker compose down
```

查看日志：

```bash
docker compose logs -f
```

默认地址：

- 管理台：`http://127.0.0.1:8080`
- 聚合路由：`http://127.0.0.1:8080/{username}`

默认管理员账号：

- 用户名：`admin`
- 密码：`admin`

管理员账户修改后会立即使当前会话失效，需要重新登录。

## 部署说明

`Dockerfile` 会在构建阶段同时生成：

- `web/dist/` 的 Vite 前端产物
- `submora` 二进制

运行时环境变量默认使用：

- `Dockerfile` 中的镜像默认值
- `backend/src/config.rs` 内置默认值

如果要覆盖默认值，直接在本地 `compose.yml` 增加 `environment:`，或通过 `docker compose --env-file ...` 提供环境变量即可。

## 常用环境变量

- `HOST` / `PORT`
- `WEB_DIST_DIR`
- `DATABASE_URL`
- `COOKIE_SECURE`
- `SESSION_TTL_MINUTES`
- `SESSION_CLEANUP_INTERVAL_SECS`
- `TRUST_PROXY_HEADERS`
- `LOGIN_MAX_ATTEMPTS`
- `LOGIN_WINDOW_SECS`
- `LOGIN_LOCKOUT_SECS`
- `PUBLIC_MAX_REQUESTS`
- `PUBLIC_WINDOW_SECS`
- `CACHE_TTL_SECS`
- `DB_MAX_CONNECTIONS`
- `FETCH_TIMEOUT_SECS`
- `DNS_CACHE_TTL_SECS`
- `DNS_CACHE_MAX_ENTRIES`
- `FETCH_HOST_OVERRIDES`
- `PINNED_CLIENT_POOL_MAX_ENTRIES`
- `CONCURRENT_LIMIT`
- `MAX_LINKS_PER_USER`
- `MAX_USERS`
- `ADMIN_USER`
- `ADMIN_PASSWORD`
- `CORS_ALLOW_ORIGIN`

## 代码校验

仓库不再提供 `Makefile`。如需直接校验源码，使用原生命令：

```bash
cargo check --workspace
cargo test --workspace
cd web && npm run check && npm run build && npm run test:unit
```

## 生产部署建议

- 对外提供服务前，必须修改默认管理员密码；不要以 `admin/admin` 直接暴露公网。
- 生产环境应放在 HTTPS 反向代理之后，并设置 `COOKIE_SECURE=true`。
- `TRUST_PROXY_HEADERS` 默认值是 `false`。只有当所有流量都经过你可控、且会清洗 `x-forwarded-for` / `x-real-ip` 的反向代理时，才应该设为 `true`。
- `data/` 是当前默认持久化边界，至少应纳入定时备份；升级前先做冷备份。
- 当前运行模型以单机 SQLite 为主，适合单节点自托管。

## 文档约定

仓库不再保留独立 `docs/` 目录，开发、测试、部署和运行说明统一维护在当前 README。
