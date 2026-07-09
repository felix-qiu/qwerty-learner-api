# Qwerty Learner API 文档

面向前后端分离二次开发的接口规范。当前仓库是纯前端 SPA（React + Vite），词典来自静态 JSON，练习记录与设置分别落在 IndexedDB / localStorage。本目录定义后端应提供的 REST API，使前端可逐步替换本地存储。

## 文档索引

| 文件 | 说明 |
|------|------|
| [API.md](./API.md) | 人类可读接口说明（推荐先读） |
| [openapi.yaml](./openapi.yaml) | OpenAPI 3.0 机器可读规范 |
| [schema.sql](./schema.sql) | PostgreSQL 14+ 完整表结构（含视图/函数） |
| [examples/frontend.ts](./examples/frontend.ts) | 前端调用示例 |
| [examples/import-dict.mjs](./examples/import-dict.mjs) | 词典批量导入脚本示例 |

## 现状 → 目标

```
现状（纯前端）                         目标（前后端分离）
─────────────────                     ─────────────────
dictionary.ts 硬编码元数据      →     GET /api/v1/dictionaries
public/dicts/*.json + fetch     →     GET /api/v1/dictionaries/{id}/words
Dexie wordRecords               →     POST/GET /api/v1/records/words
Dexie chapterRecords            →     POST/GET /api/v1/records/chapters
Dexie reviewRecords             →     POST/GET /api/v1/reviews
localStorage (Jotai)            →     GET/PUT /api/v1/settings
客户端聚合统计                   →     GET /api/v1/stats/*
本地 gzip 导入导出               →     POST /api/v1/data/export|import
有道发音直连                     →     GET /api/v1/pronunciation（可选代理）
```

## 约定

- **Base URL**：`https://{host}/api/v1`
- **协议**：HTTPS + JSON（`Content-Type: application/json`）
- **鉴权**：Bearer JWT（`Authorization: Bearer <token>`）；词典只读接口可匿名
- **时间戳**：Unix 秒（UTC），与现有 `IWordRecord.timeStamp` 一致
- **章节**：每章固定 `CHAPTER_LENGTH = 20` 词，章节号从 `0` 起；复习模式 `chapter = -1`
- **错误格式**：

```json
{
  "code": "NOT_FOUND",
  "message": "Dictionary not found",
  "details": {}
}
```

## 资源域一览

| 域 | 前缀 | 权限 |
|----|------|------|
| 认证 | `/auth` | 公开 |
| 词典 | `/dictionaries` | 读公开 / 写需管理员 |
| 练习记录 | `/records` | 登录用户 |
| 复习 | `/reviews` | 登录用户 |
| 错题 | `/error-book` | 登录用户 |
| 统计 | `/stats` | 登录用户 |
| 设置 | `/settings` | 登录用户 |
| 数据迁移 | `/data` | 登录用户 |
| 发音 | `/pronunciation` | 公开（可选） |

## 迁移建议（分阶段）

1. **Phase 0**：后端只提供词典元数据 + 按章取词；前端继续用本地记录
2. **Phase 1**：用户体系 + 练习记录云同步；保留本地 Dexie 作离线缓存
3. **Phase 2**：设置同步、错题本 / 分析走服务端聚合
4. **Phase 3**：导入导出、复习算法服务端化、发音代理

## 数据库表一览（`schema.sql`）

| 表 / 视图 | 用途 | 对应 API |
|-----------|------|----------|
| `users` / `refresh_tokens` / `user_tokens` | 账号与鉴权 | `/auth/*` |
| `dictionaries` / `words` | 词典元数据与单词 | `/dictionaries` |
| `word_records` / `chapter_records` | 练习记录 | `/records/*` |
| `review_records` | 智能复习 | `/reviews` |
| `user_settings` | 用户偏好 | `/settings` |
| `sound_resources` | 音效资源目录 | settings 内引用 |
| `pronunciation_cache` | 发音缓存 | `/pronunciation` |
| `data_jobs` | 导入导出审计 | `/data/*` |
| `v_error_words` | 错题聚合视图 | `/error-book` |
| `v_dict_stats` / `v_chapter_stats` | 进度与章节统计 | `/stats/*` |
| `v_user_summary` / `v_daily_word_activity` | 总览与按日分析 | `/stats/summary` `/stats/analysis` |

另含函数：`get_chapter_words`、`sync_dictionary_length`、`rank_review_words`、`activity_level`。

## 本地对照源码

| 概念 | 源码位置 |
|------|----------|
| Word / Dictionary | `src/typings/index.ts`, `src/typings/resource.ts` |
| 记录模型 | `src/utils/db/record.ts` |
| Dexie Schema | `src/utils/db/index.ts` |
| 词典注册表 | `src/resources/dictionary.ts` |
| 章节长度 | `src/constants/index.ts` → `CHAPTER_LENGTH = 20` |
| 复习算法 | `src/utils/db/review-record.ts` |
| 用户设置 | `src/store/index.ts` |
