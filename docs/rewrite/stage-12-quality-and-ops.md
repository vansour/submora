# Stage 12 Quality And Ops Closure

阶段十二用于把前面几个阶段的能力沉淀成长期可维护的资产，而不是继续只做功能补丁。

本阶段聚焦四件事：

- 补最关键的回归测试
- 增强关键路径结构化日志
- 同步 README / 计划文档
- 清理已经失效的遗留代码

## 完成项

### 1. 回归测试落地

- `packages/core/tests/core_behaviors.rs`
  - 覆盖用户名、密码、URL 与链接归一化等纯函数行为。
- `backend/tests/integration_flows.rs`
  - 覆盖管理员登录与账户更新流
  - 覆盖订阅组创建、保存链接、公共聚合路由 cache `miss/hit`
  - 覆盖 diagnostics / cache 管理 API
  - 覆盖大响应体抓取失败时的 diagnostics 记录

这些测试现在直接进入：

- `cargo test --workspace`

### 2. 关键日志字段增强

- `backend/src/security.rs`
  - 登录限流与公共路由限流命中时，日志带 `scope`、`key`、`retry_after_secs`。
- `backend/src/routes/public.rs`
  - 公共聚合响应会记录 `username`、`cache_state`、`generated_at`、`expires_at`、`link_count`。
- `backend/src/routes/auth.rs`
  - 管理员登录、登出、账户更新会记录结构化事件。
- `backend/src/routes/users.rs`
  - 创建订阅组、删除订阅组、更新链接、更新排序、刷新/清空缓存都带稳定字段输出。

### 3. 文档同步

- `README.md`
  - 补齐 diagnostics / cache 面板说明
  - 补齐账户更新行为
  - 补齐 cache / diagnostics 相关 API
  - 补齐 `cargo test --workspace` / `make test`
- `docs/optimization-plan.md`
  - 回写阶段 D 完成状态
- `docs/rewrite/stage-7-security-and-observability.md`
  - 修正早期文档中对前端接入时机的偏差描述

### 4. 遗留代码清理

- 删除未被引用的 `frontend/src/components/console/view.rs`
- 删除未被使用的前端 `run_public_route` 辅助 API

## 关键文件

- `packages/core/tests/core_behaviors.rs`
- `backend/tests/integration_flows.rs`
- `backend/src/security.rs`
- `backend/src/routes/auth.rs`
- `backend/src/routes/public.rs`
- `backend/src/routes/users.rs`
- `README.md`
- `docs/optimization-plan.md`

## 验证

本地应通过：

```bash
cargo fmt --all
cargo check --workspace
cargo check -p submora-web --target wasm32-unknown-unknown
cargo test --workspace
```

阶段十二完成后，仓库的默认维护姿态变成：

- 关键纯函数和核心业务流有回归测试
- 关键运行路径有稳定日志字段可用于排障
- README、阶段文档与当前实现重新对齐
