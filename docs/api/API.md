# Qwerty Learner REST API

> Base URL: `/api/v1`  
> 版本：`1.0.0`  
> 与前端类型对齐：`src/typings/*`、`src/utils/db/record.ts`

---

## 目录

1. [通用约定](#1-通用约定)
2. [认证 Auth](#2-认证-auth)
3. [词典 Dictionaries](#3-词典-dictionaries)
4. [练习记录 Records](#4-练习记录-records)
5. [复习 Reviews](#5-复习-reviews)
6. [错题本 Error Book](#6-错题本-error-book)
7. [统计分析 Stats](#7-统计分析-stats)
8. [用户设置 Settings](#8-用户设置-settings)
9. [数据导入导出 Data](#9-数据导入导出-data)
10. [发音代理 Pronunciation](#10-发音代理-pronunciation可选)
11. [数据模型附录](#11-数据模型附录)

---

## 1. 通用约定

### 1.1 请求头

| Header | 说明 |
|--------|------|
| `Authorization` | `Bearer <access_token>`（需登录的接口） |
| `Content-Type` | `application/json` |
| `Accept` | `application/json` |

### 1.2 统一响应包

推荐直接返回资源体；若需统一包装：

```json
{
  "data": {},
  "meta": { "requestId": "uuid" }
}
```

列表接口推荐：

```json
{
  "items": [],
  "total": 100,
  "page": 1,
  "pageSize": 20
}
```

### 1.3 错误码

| HTTP | code | 说明 |
|------|------|------|
| 400 | `BAD_REQUEST` | 参数错误 |
| 401 | `UNAUTHORIZED` | 未登录或 token 失效 |
| 403 | `FORBIDDEN` | 无权限 |
| 404 | `NOT_FOUND` | 资源不存在 |
| 409 | `CONFLICT` | 冲突（如邮箱已注册） |
| 422 | `VALIDATION_ERROR` | 校验失败 |
| 429 | `RATE_LIMITED` | 限流 |
| 500 | `INTERNAL_ERROR` | 服务端错误 |

### 1.4 分页查询参数

| 参数 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `page` | integer | 1 | 页码，从 1 起 |
| `pageSize` | integer | 20 | 每页条数，最大 100 |
| `sort` | string | — | 如 `timeStamp:desc` |

---

## 2. 认证 Auth

当前前端无用户体系。前后端分离后建议引入轻量账号，用于云同步练习记录与设置。

### 2.1 注册

`POST /auth/register`

**Request**

```json
{
  "email": "user@example.com",
  "password": "string(8+)",
  "displayName": "可选昵称"
}
```

**Response `201`**

```json
{
  "user": {
    "id": "usr_xxx",
    "email": "user@example.com",
    "displayName": "可选昵称",
    "createdAt": 1719900000
  },
  "accessToken": "jwt...",
  "refreshToken": "jwt...",
  "expiresIn": 3600
}
```

### 2.2 登录

`POST /auth/login`

**Request**

```json
{
  "email": "user@example.com",
  "password": "string"
}
```

**Response `200`**：同注册响应。

### 2.3 刷新 Token

`POST /auth/refresh`

```json
{ "refreshToken": "jwt..." }
```

### 2.4 当前用户

`GET /auth/me` — 需登录

```json
{
  "id": "usr_xxx",
  "email": "user@example.com",
  "displayName": "昵称",
  "createdAt": 1719900000
}
```

### 2.5 登出

`POST /auth/logout` — 需登录（服务端可作废 refresh token）

---

## 3. 词典 Dictionaries

替代 `src/resources/dictionary.ts` + `public/dicts/*.json`。

### 3.1 词典列表

`GET /dictionaries`

**Query**

| 参数 | 类型 | 说明 |
|------|------|------|
| `languageCategory` | string | `en` / `ja` / `de` / `code` / `kk` / `id` |
| `category` | string | 如 `中国考试`、`代码练习` |
| `tag` | string | 标签过滤 |
| `q` | string | 名称/描述模糊搜索 |

**Response `200`**

```json
{
  "items": [
    {
      "id": "cet4",
      "name": "CET-4",
      "description": "大学英语四级词库",
      "category": "中国考试",
      "tags": ["大学英语"],
      "length": 2607,
      "chapterCount": 131,
      "language": "en",
      "languageCategory": "en",
      "defaultPronIndex": null
    }
  ],
  "total": 100
}
```

> `chapterCount = ceil(length / 20)`，与前端 `calcChapterCount` 一致。

### 3.2 词典详情

`GET /dictionaries/{dictId}`

**Response `200`**：单个 Dictionary 对象。  
**Response `404`**：词典不存在。

### 3.3 按章获取单词

`GET /dictionaries/{dictId}/words`

替代 `wordListFetcher` + `slice(chapter * 20, (chapter+1) * 20)`。

**Query**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `chapter` | integer | 否 | 章节号，从 0 起；不传则返回全部（大词库慎用） |
| `offset` | integer | 否 | 与 `limit` 二选一分页方式 |
| `limit` | integer | 否 | 默认 20，最大 500 |

**Response `200`（按章）**

```json
{
  "dictId": "cet4",
  "chapter": 0,
  "chapterCount": 131,
  "total": 2607,
  "words": [
    {
      "index": 0,
      "name": "cancel",
      "trans": ["取消， 撤销； 删去"],
      "usphone": "'kænsl",
      "ukphone": "'kænsl",
      "notation": null
    }
  ]
}
```

> `index` 为该词在整本词典中的全局下标（与前端 `WordWithIndex` 一致）。

### 3.4 按单词名查询（可选）

`GET /dictionaries/{dictId}/words/{wordName}`

用于错题本回填释义。

### 3.5 管理端：创建/更新词典（需管理员）

```
POST   /dictionaries
PUT    /dictionaries/{dictId}
POST   /dictionaries/{dictId}/words:bulk   # 批量写入单词
DELETE /dictionaries/{dictId}
```

**批量写入 Body**

```json
{
  "mode": "replace",
  "words": [
    { "name": "cancel", "trans": ["取消"], "usphone": "'kænsl", "ukphone": "'kænsl" }
  ]
}
```

`mode`：`replace`（覆盖整本）| `append`（追加）。

---

## 4. 练习记录 Records

对应 Dexie 表 `wordRecords` / `chapterRecords`。

### 4.1 提交单词练习记录

`POST /records/words` — 需登录

对应 `useSaveWordRecord`。

**Request**

```json
{
  "word": "cancel",
  "dict": "cet4",
  "chapter": 0,
  "timing": [120, 95, 110, 88, 102, 90],
  "wrongCount": 1,
  "mistakes": {
    "0": ["x"],
    "2": ["n", "m"]
  },
  "timeStamp": 1719900123
}
```

| 字段 | 说明 |
|------|------|
| `chapter` | 正常章节 ≥0；复习模式传 `-1`；错题练习可为 `null` |
| `timing` | 相邻字母输入时间差（ms），长度通常为 `word.length` |
| `mistakes` | key 为字母下标（字符串），value 为错误按键数组 |
| `timeStamp` | 可选；不传则服务端生成 |

**Response `201`**

```json
{
  "id": 1001,
  "word": "cancel",
  "dict": "cet4",
  "chapter": 0,
  "timing": [120, 95, 110, 88, 102, 90],
  "wrongCount": 1,
  "mistakes": { "0": ["x"], "2": ["n", "m"] },
  "timeStamp": 1719900123,
  "totalTime": 605
}
```

> `totalTime = sum(timing)`，对应前端 `WordRecord.totalTime`。

### 4.2 批量提交单词记录

`POST /records/words:batch` — 需登录

离线同步场景：

```json
{
  "items": [ { "...WordRecord" }, { "...WordRecord" } ]
}
```

**Response `201`**

```json
{ "ids": [1001, 1002], "accepted": 2, "rejected": 0 }
```

### 4.3 查询单词记录

`GET /records/words` — 需登录

**Query**

| 参数 | 说明 |
|------|------|
| `dict` | 词典 id |
| `chapter` | 章节号 |
| `word` | 精确匹配单词 |
| `wrongOnly` | `true` 时仅 `wrongCount > 0` |
| `from` / `to` | `timeStamp` 区间（Unix 秒） |
| `page` / `pageSize` / `sort` | 分页排序 |

### 4.4 删除某词在某词典下的全部记录

`DELETE /records/words` — 需登录

**Query**：`word`（必填）+ `dict`（必填）

对应 `useDeleteWordRecord`。

**Response `200`**

```json
{ "deletedCount": 3 }
```

### 4.5 提交章节练习记录

`POST /records/chapters` — 需登录

对应 `useSaveChapterRecord`。

**Request**

```json
{
  "dict": "cet4",
  "chapter": 0,
  "time": 185,
  "correctCount": 312,
  "wrongCount": 8,
  "wordCount": 22,
  "correctWordIndexes": [0, 1, 3, 4, 5],
  "wordNumber": 20,
  "wordRecordIds": [1001, 1002, 1003],
  "timeStamp": 1719900300
}
```

| 字段 | 说明 |
|------|------|
| `time` | 章节用时（秒） |
| `correctCount` | 正确按键次数 |
| `wrongCount` | 错误按键次数（整词清空只记 1 次） |
| `wordCount` | 实际输入单词次数（含循环，可 >20） |
| `correctWordIndexes` | 一次打对未犯错的单词下标 |
| `wordNumber` | 本章单词总数（通常 20） |
| `wordRecordIds` | 关联的单词记录 id |
| `chapter` | 复习模式传 `-1` |

**Response `201`**：带 `id` 的 ChapterRecord；可附计算字段：

```json
{
  "id": 501,
  "wpm": 7,
  "wordAccuracy": 25,
  "dict": "cet4",
  "chapter": 0,
  "time": 185,
  "correctCount": 312,
  "wrongCount": 8,
  "wordCount": 22,
  "correctWordIndexes": [0, 1, 3, 4, 5],
  "wordNumber": 20,
  "wordRecordIds": [1001, 1002, 1003],
  "timeStamp": 1719900300
}
```

> `wpm = round(wordCount / time * 60)`  
> `wordAccuracy = round(correctWordIndexes.length / wordNumber * 100)`

### 4.6 查询章节记录

`GET /records/chapters` — 需登录

**Query**：`dict`、`chapter`、`from`、`to`、分页参数。

---

## 5. 复习 Reviews

对应 `reviewRecords` 与 `generateNewWordReviewRecord`。

### 5.1 获取某词典最新未完成复习

`GET /reviews/latest?dict={dictId}` — 需登录

对应 `useGetLatestReviewRecord`：取该词典最新一条且 `isFinished === false` 的记录；若最新已完成则返回 `null`。

**Response `200`**

```json
{
  "id": 88,
  "dict": "cet4",
  "index": 3,
  "createTime": 1719800000,
  "isFinished": false,
  "words": [
    { "name": "cancel", "trans": ["取消"], "usphone": "'kænsl", "ukphone": "'kænsl" }
  ]
}
```

或 `{ "record": null }`。

### 5.2 基于错题生成新复习

`POST /reviews` — 需登录

服务端可自行聚合错题，也可接收客户端已算好的 errorData。

**方式 A：服务端聚合（推荐）**

```json
{ "dict": "cet4" }
```

**方式 B：客户端传入错题数据**

```json
{
  "dict": "cet4",
  "errorData": [
    {
      "word": "cancel",
      "errorCount": 5,
      "latestErrorTime": 1719800000,
      "originData": {
        "name": "cancel",
        "trans": ["取消"],
        "usphone": "'kænsl",
        "ukphone": "'kænsl"
      }
    }
  ]
}
```

**排序算法**（与 `review-record.ts` 一致）：

1. 分别按 `errorCount`、`latestErrorTime` 升序排名打分
2. `score = errorCountScore * 0.6 + latestErrorTimeScore * 0.4`
3. 按 score 升序得到复习词序（错得少且久远的靠前）

**Response `201`**：新建的 ReviewRecord（`index=0`, `isFinished=false`）。

### 5.3 更新复习进度

`PATCH /reviews/{reviewId}` — 需登录

```json
{
  "index": 10,
  "isFinished": false,
  "words": null
}
```

练习过程中推进 `index`；完成时设 `isFinished: true`。`words` 一般不改。

### 5.4 复习列表

`GET /reviews?dict={dictId}` — 需登录

---

## 6. 错题本 Error Book

对应 ErrorBook 页与 `useErrorWords`。

### 6.1 全局错题列表

`GET /error-book` — 需登录

聚合当前用户所有 `wrongCount > 0` 的单词记录。

**Query**：`dict`（可选过滤）、`page`、`pageSize`、`sort`（如 `wrongCount:desc`）

**Response `200`**

```json
{
  "items": [
    {
      "word": "cancel",
      "dict": "cet4",
      "wrongCount": 5,
      "latestErrorTime": 1719900000,
      "errorLetters": { "0": 2, "2": 3 },
      "errorChar": ["c", "n"],
      "originData": {
        "name": "cancel",
        "trans": ["取消， 撤销； 删去"],
        "usphone": "'kænsl",
        "ukphone": "'kænsl"
      },
      "records": []
    }
  ],
  "total": 42
}
```

| 字段 | 说明 |
|------|------|
| `wrongCount` | 该词所有记录的 `wrongCount` 之和 |
| `errorLetters` | 字母下标 → 错误次数 |
| `errorChar` | 按错误次数降序的易错字母 |
| `records` | 可选；`?includeRecords=true` 时返回原始记录 |

### 6.2 某词典错题

`GET /dictionaries/{dictId}/error-words` — 需登录

与 Gallery 词典详情「错题」Tab 对齐，结构同 `TErrorWordData[]`。

---

## 7. 统计分析 Stats

将客户端 Dexie 聚合上移到服务端。

### 7.1 章节统计

`GET /stats/chapters?dict={dictId}&chapter={n}` — 需登录

对应 `useChapterStats`：

```json
{
  "dict": "cet4",
  "chapter": 0,
  "exerciseCount": 5,
  "avgWrongWordCount": 2.4,
  "avgWrongInputCount": 3.2
}
```

### 7.2 词典练习进度

`GET /stats/dictionaries/{dictId}` — 需登录

对应 `useDictStats`：

```json
{
  "dict": "cet4",
  "exercisedChapterCount": 12,
  "chapterCount": 131
}
```

### 7.3 分析页总览

`GET /stats/analysis?from={unix}&to={unix}` — 需登录

对应 `useWordStats`：

`from` / `to` 仍使用 Unix 秒且区间包含首尾；所有按日指标统一按
`Asia/Shanghai` 自然日分组并补齐区间内无记录日期。

```json
{
  "isEmpty": false,
  "exerciseRecord": [
    { "date": "2026-07-01", "count": 5, "level": 2 }
  ],
  "wordRecord": [
    { "date": "2026-07-01", "count": 18, "level": 3 }
  ],
  "wpmRecord": [["2026-07-01", 42]],
  "accuracyRecord": [["2026-07-01", 96]],
  "wrongTimeRecord": [
    { "name": "A", "value": 12 },
    { "name": "E", "value": 8 }
  ]
}
```

**计算规则**（与前端一致）：

| 指标 | 公式 |
|------|------|
| `exerciseRecord.count` | 当日单词记录条数 |
| `wordRecord.count` | 当日练习词去重数 |
| `wpm` | `round(词数不去重 / (总 timing ms / 1000 / 60))` |
| `accuracy` | `round(字母总长 / (字母总长 + wrongCount) * 100)` |
| `level` | 0 / `<4→1` / `<8→2` / `<12→3` / `≥12→4` |
| `wrongTimeRecord` | 汇总 `mistakes` 中所有错误键（大写）频次 |

### 7.4 用户总览（捐赠卡等）

`GET /stats/summary` — 需登录

```json
{
  "wordRecordCount": 1200,
  "chapterRecordCount": 80,
  "totalTimeSeconds": 36000,
  "firstPracticeAt": 1700000000
}
```

---

## 8. 用户设置 Settings

替代 Jotai `atomWithStorage` / `atomForConfig`（`src/store/index.ts`）。

### 8.1 获取设置

`GET /settings` — 需登录

**Response `200`**

```json
{
  "currentDict": "cet4",
  "currentChapter": 0,
  "loopWordConfig": { "times": 1 },
  "keySoundsConfig": {
    "isOpen": true,
    "isOpenClickSound": true,
    "volume": 1,
    "resource": null
  },
  "hintSoundsConfig": {
    "isOpen": true,
    "volume": 1,
    "isOpenWrongSound": true,
    "isOpenCorrectSound": true,
    "wrongResource": null,
    "correctResource": null
  },
  "pronunciation": {
    "isOpen": true,
    "volume": 1,
    "type": "us",
    "name": "美音",
    "isLoop": false,
    "isTransRead": false,
    "transVolume": 1,
    "rate": 1
  },
  "fontsize": { "foreignFont": 48, "translateFont": 18 },
  "randomConfig": { "isOpen": false },
  "phoneticConfig": { "isOpen": true, "type": "us" },
  "wordDictationConfig": {
    "isOpen": false,
    "type": "hideAll",
    "openBy": "auto"
  },
  "isShowPrevAndNextWord": true,
  "isIgnoreCase": true,
  "isShowAnswerOnHover": true,
  "isTextSelectable": false,
  "isOpenDarkMode": false,
  "dismissStartCardDate": null,
  "hasSeenEnhancedPromotion": false
}
```

### 8.2 全量更新

`PUT /settings` — 需登录

Body 为完整设置对象。

### 8.3 部分更新

`PATCH /settings` — 需登录

```json
{
  "currentDict": "cet6",
  "currentChapter": 3,
  "pronunciation": { "type": "uk", "name": "英音" }
}
```

深合并对象字段；布尔/标量直接覆盖。

`resource`、`wrongResource`、`correctResource` 与 `dismissStartCardDate`
允许显式传 `null`；其他已定义字段传 `null` 时返回 `422`。文档外字段按
OpenAPI `additionalProperties` 语义保留，PATCH 时同样参与深合并。

---

## 9. 数据导入导出 Data

对应 `src/utils/db/data-export.ts`（原 Dexie + pako gzip）。

### 9.1 导出

`POST /data/export` — 需登录

**Request（可选过滤）**

```json
{
  "include": ["wordRecords", "chapterRecords", "reviewRecords", "settings"]
}
```

**Response `200`**

- `Content-Type: application/gzip`
- `Content-Disposition: attachment; filename="Qwerty-Learner-User-Data-YYYY-MM-DD.gz"`
- Body：gzip 压缩的 JSON，结构建议：

```json
{
  "version": 3,
  "exportedAt": 1719900000,
  "userId": "usr_xxx",
  "wordRecords": [],
  "chapterRecords": [],
  "reviewRecords": [],
  "settings": {}
}
```

### 9.2 导入

`POST /data/import` — 需登录  
`Content-Type: multipart/form-data`，字段 `file`（`.gz`）

**Query**

| 参数 | 默认 | 说明 |
|------|------|------|
| `mode` | `replace` | `replace` 清空后导入；`merge` 按 id/业务键合并 |

**Response `200`**

```json
{
  "wordCount": 1200,
  "chapterCount": 80,
  "reviewCount": 5,
  "settingsUpdated": true
}
```

---

## 10. 发音代理 Pronunciation（可选）

当前前端直连有道：`https://dict.youdao.com/dictvoice?audio=`。  
若需规避 CORS / 稳定性，可由后端代理。

`GET /pronunciation`

| 参数 | 说明 |
|------|------|
| `word` | 单词或短语 |
| `type` | `us` / `uk` / `ja` / `de` / … 映射到有道 `type` 参数 |

**Response**：`audio/mpeg` 流，或 `302` 到 CDN。

---

## 11. 数据模型附录

### 11.1 Word

```ts
type Word = {
  name: string
  trans: string[]
  usphone: string
  ukphone: string
  notation?: string
}
```

### 11.2 Dictionary

```ts
type Dictionary = {
  id: string
  name: string
  description: string
  category: string
  tags: string[]
  length: number
  chapterCount: number          // ceil(length / 20)
  language: LanguageType        // en|romaji|zh|ja|code|de|kk|hapin|id
  languageCategory: LanguageCategoryType // en|ja|de|code|kk|id
  defaultPronIndex?: number
}
```

### 11.3 WordRecord

```ts
type WordRecord = {
  id: number
  word: string
  timeStamp: number             // Unix 秒
  dict: string
  chapter: number | null        // -1 = 复习
  timing: number[]              // ms 差值
  wrongCount: number
  mistakes: Record<string, string[]>
}
```

### 11.4 ChapterRecord

```ts
type ChapterRecord = {
  id: number
  dict: string
  chapter: number | null
  timeStamp: number
  time: number                  // 秒
  correctCount: number
  wrongCount: number
  wordCount: number
  correctWordIndexes: number[]
  wordNumber: number
  wordRecordIds: number[]
}
```

### 11.5 ReviewRecord

```ts
type ReviewRecord = {
  id: number
  dict: string
  index: number
  createTime: number
  isFinished: boolean
  words: Word[]
}
```

### 11.6 枚举速查

| 名称 | 值 |
|------|-----|
| `PronunciationType` | `us` `uk` `romaji` `zh` `ja` `de` `hapin` `kk` `id` |
| `WordDictationType` | `hideAll` `hideVowel` `hideConsonant` `randomHide` |
| `WordDictationOpenBy` | `user` `auto` |
| `LoopWordTimes` | `1` `3` `5` `8` 或极大整数（无限） |
| `CHAPTER_LENGTH` | `20` |

### 11.7 前端迁移对照表

| 前端现状 | API |
|----------|-----|
| `idDictionaryMap` / `dictionaries` | `GET /dictionaries` |
| `wordListFetcher(url)` | `GET /dictionaries/{id}/words?chapter=` |
| `db.wordRecords.add` | `POST /records/words` |
| `db.chapterRecords.add` | `POST /records/chapters` |
| `db.wordRecords.where(...).delete` | `DELETE /records/words?word=&dict=` |
| `generateNewWordReviewRecord` | `POST /reviews` |
| `useGetLatestReviewRecord` | `GET /reviews/latest` |
| `putWordReviewRecord` | `PATCH /reviews/{id}` |
| `useErrorWords` / ErrorBook | `GET /error-book`、`GET /dictionaries/{id}/error-words` |
| `useChapterStats` | `GET /stats/chapters` |
| `useDictStats` | `GET /stats/dictionaries/{id}` |
| `useWordStats` | `GET /stats/analysis` |
| Jotai storage atoms | `GET/PUT/PATCH /settings` |
| `exportDatabase` / `importDatabase` | `POST /data/export`、`POST /data/import` |
