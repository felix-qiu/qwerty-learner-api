#!/usr/bin/env node
/**
 * 将 public/dicts/*.json + src/resources/dictionary.ts 中的元数据
 * 导入到后端 API（管理员接口）。
 *
 * 用法：
 *   export API_BASE=http://localhost:3000/api/v1
 *   export ADMIN_TOKEN=eyJ...
 *   node docs/api/examples/import-dict.mjs
 *
 * 说明：
 *   - 元数据默认从内嵌的最小示例读取；完整导入请先导出 dictionary 列表为 JSON，
 *     或改用 --meta 指向自行生成的 dictionaries.json。
 *   - 单词文件默认读取项目 public/dicts/ 下与 url 对应的 JSON。
 */

import { readFile, readdir } from 'node:fs/promises'
import { resolve, dirname, basename } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const ROOT = resolve(__dirname, '../../..')
const API_BASE = process.env.API_BASE ?? 'http://localhost:3000/api/v1'
const ADMIN_TOKEN = process.env.ADMIN_TOKEN
const DICTS_DIR = resolve(ROOT, 'public/dicts')

if (!ADMIN_TOKEN) {
  console.error('请设置环境变量 ADMIN_TOKEN')
  process.exit(1)
}

async function api(path, { method = 'GET', body } = {}) {
  const res = await fetch(`${API_BASE}${path}`, {
    method,
    headers: {
      Authorization: `Bearer ${ADMIN_TOKEN}`,
      Accept: 'application/json',
      ...(body ? { 'Content-Type': 'application/json' } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  })
  if (!res.ok) {
    const text = await res.text()
    throw new Error(`${method} ${path} → ${res.status}: ${text}`)
  }
  if (res.status === 204) return null
  return res.json()
}

/** 从 public/dicts 扫描，用文件名推断 id（演示用；生产请用完整元数据） */
async function discoverFromFiles() {
  const files = (await readdir(DICTS_DIR)).filter((f) => f.endsWith('.json'))
  return files.map((file) => {
    const id = basename(file, '.json')
      .replace(/_T$/i, '')
      .toLowerCase()
    return {
      id,
      name: id,
      description: `Imported from ${file}`,
      category: 'imported',
      tags: ['imported'],
      language: 'en',
      languageCategory: 'en',
      url: `/dicts/${file}`,
    }
  })
}

async function loadMeta() {
  const metaPath = process.argv.find((a) => a.startsWith('--meta='))?.slice(7)
  if (metaPath) {
    const raw = await readFile(resolve(ROOT, metaPath), 'utf8')
    return JSON.parse(raw)
  }
  console.warn('未指定 --meta=dictionaries.json，将按 public/dicts 文件名粗略导入（仅演示）')
  return discoverFromFiles()
}

async function importOne(meta) {
  const fileName = meta.url?.replace(/^\/dicts\//, '') ?? `${meta.id}.json`
  const filePath = resolve(DICTS_DIR, fileName)

  let words
  try {
    words = JSON.parse(await readFile(filePath, 'utf8'))
  } catch (e) {
    console.warn(`跳过 ${meta.id}：无法读取 ${filePath}`)
    return
  }

  if (!Array.isArray(words)) {
    console.warn(`跳过 ${meta.id}：JSON 不是数组`)
    return
  }

  const normalized = words.map((w) => ({
    name: w.name,
    trans: Array.isArray(w.trans) ? w.trans : [String(w.trans ?? '')],
    usphone: w.usphone ?? '',
    ukphone: w.ukphone ?? '',
    notation: w.notation ?? null,
  }))

  await api('/dictionaries', {
    method: 'POST',
    body: {
      id: meta.id,
      name: meta.name,
      description: meta.description ?? '',
      category: meta.category ?? '未分类',
      tags: meta.tags ?? [],
      language: meta.language ?? 'en',
      languageCategory: meta.languageCategory ?? 'en',
      defaultPronIndex: meta.defaultPronIndex ?? null,
    },
  }).catch(async (err) => {
    // 已存在则更新
    if (String(err.message).includes('409') || String(err.message).includes('CONFLICT')) {
      await api(`/dictionaries/${encodeURIComponent(meta.id)}`, {
        method: 'PUT',
        body: {
          id: meta.id,
          name: meta.name,
          description: meta.description ?? '',
          category: meta.category ?? '未分类',
          tags: meta.tags ?? [],
          language: meta.language ?? 'en',
          languageCategory: meta.languageCategory ?? 'en',
          defaultPronIndex: meta.defaultPronIndex ?? null,
        },
      })
      return
    }
    throw err
  })

  const result = await api(`/dictionaries/${encodeURIComponent(meta.id)}/words:bulk`, {
    method: 'POST',
    body: { mode: 'replace', words: normalized },
  })

  console.log(
    `✓ ${meta.id}: ${result?.written ?? normalized.length} words, chapters=${result?.chapterCount ?? Math.ceil(normalized.length / 20)}`,
  )
}

async function main() {
  const list = await loadMeta()
  console.log(`准备导入 ${list.length} 本词典 → ${API_BASE}`)
  for (const meta of list) {
    try {
      await importOne(meta)
    } catch (e) {
      console.error(`✗ ${meta.id}:`, e.message)
    }
  }
}

main()
