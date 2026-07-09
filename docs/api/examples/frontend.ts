/**
 * Qwerty Learner API — 前端调用示例
 *
 * 用法：将 BASE_URL / token 换成实际值。
 * 可逐步替换：
 *   - wordListFetcher → api.getChapterWords
 *   - useSaveWordRecord → api.createWordRecord
 *   - Jotai settings → api.getSettings / patchSettings
 */

const BASE_URL = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:3000/api/v1'

type Word = {
  name: string
  trans: string[]
  usphone: string
  ukphone: string
  notation?: string
}

type WordWithIndex = Word & { index: number }

type Dictionary = {
  id: string
  name: string
  description: string
  category: string
  tags: string[]
  length: number
  chapterCount: number
  language: string
  languageCategory: string
  defaultPronIndex?: number | null
}

type LetterMistakes = Record<string, string[]>

type WordRecordCreate = {
  word: string
  dict: string
  chapter: number | null
  timing: number[]
  wrongCount: number
  mistakes: LetterMistakes
  timeStamp?: number
}

type ChapterRecordCreate = {
  dict: string
  chapter: number | null
  time: number
  correctCount: number
  wrongCount: number
  wordCount: number
  correctWordIndexes: number[]
  wordNumber: number
  wordRecordIds: number[]
  timeStamp?: number
}

type UserSettings = {
  currentDict: string
  currentChapter: number
  // ... 其余字段见 API.md §8
  [key: string]: unknown
}

class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

async function request<T>(
  path: string,
  options: RequestInit & { token?: string | null } = {},
): Promise<T> {
  const { token, headers, ...rest } = options
  const res = await fetch(`${BASE_URL}${path}`, {
    ...rest,
    headers: {
      Accept: 'application/json',
      ...(rest.body ? { 'Content-Type': 'application/json' } : {}),
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...headers,
    },
  })

  if (!res.ok) {
    let code = 'HTTP_ERROR'
    let message = res.statusText
    try {
      const err = await res.json()
      code = err.code ?? code
      message = err.message ?? message
    } catch {
      /* ignore */
    }
    throw new ApiError(res.status, code, message)
  }

  if (res.status === 204) return undefined as T
  const ct = res.headers.get('content-type') ?? ''
  if (ct.includes('application/json')) return res.json() as Promise<T>
  return res as unknown as T
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

export async function login(email: string, password: string) {
  return request<{
    user: { id: string; email: string; displayName?: string }
    accessToken: string
    refreshToken: string
    expiresIn: number
  }>('/auth/login', {
    method: 'POST',
    body: JSON.stringify({ email, password }),
  })
}

// ---------------------------------------------------------------------------
// Dictionaries（可匿名）
// ---------------------------------------------------------------------------

export async function listDictionaries(params?: {
  languageCategory?: string
  category?: string
  tag?: string
  q?: string
}) {
  const qs = new URLSearchParams()
  if (params?.languageCategory) qs.set('languageCategory', params.languageCategory)
  if (params?.category) qs.set('category', params.category)
  if (params?.tag) qs.set('tag', params.tag)
  if (params?.q) qs.set('q', params.q)
  const query = qs.toString()
  return request<{ items: Dictionary[]; total: number }>(
    `/dictionaries${query ? `?${query}` : ''}`,
  )
}

/** 替代 wordListFetcher + slice(chapter * 20, ...) */
export async function getChapterWords(dictId: string, chapter: number) {
  return request<{
    dictId: string
    chapter: number
    chapterCount: number
    total: number
    words: WordWithIndex[]
  }>(`/dictionaries/${encodeURIComponent(dictId)}/words?chapter=${chapter}`)
}

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

/** 替代 useSaveWordRecord → db.wordRecords.add */
export async function createWordRecord(token: string, body: WordRecordCreate) {
  return request<{ id: number } & WordRecordCreate & { totalTime: number }>(
    '/records/words',
    { method: 'POST', token, body: JSON.stringify(body) },
  )
}

/** 替代 useSaveChapterRecord → db.chapterRecords.add */
export async function createChapterRecord(token: string, body: ChapterRecordCreate) {
  return request<{ id: number } & ChapterRecordCreate>('/records/chapters', {
    method: 'POST',
    token,
    body: JSON.stringify(body),
  })
}

/** 替代 useDeleteWordRecord */
export async function deleteWordRecords(token: string, word: string, dict: string) {
  const qs = new URLSearchParams({ word, dict })
  return request<{ deletedCount: number }>(`/records/words?${qs}`, {
    method: 'DELETE',
    token,
  })
}

// ---------------------------------------------------------------------------
// Reviews
// ---------------------------------------------------------------------------

export async function getLatestReview(token: string, dict: string) {
  return request<{ record: null | { id: number; dict: string; index: number; words: Word[] } }>(
    `/reviews/latest?dict=${encodeURIComponent(dict)}`,
    { token },
  )
}

export async function createReview(token: string, dict: string) {
  return request('/reviews', {
    method: 'POST',
    token,
    body: JSON.stringify({ dict }),
  })
}

export async function patchReview(
  token: string,
  reviewId: number,
  patch: { index?: number; isFinished?: boolean },
) {
  return request(`/reviews/${reviewId}`, {
    method: 'PATCH',
    token,
    body: JSON.stringify(patch),
  })
}

// ---------------------------------------------------------------------------
// Stats / Error book / Settings
// ---------------------------------------------------------------------------

export async function getAnalysisStats(token: string, from: number, to: number) {
  return request(`/stats/analysis?from=${from}&to=${to}`, { token })
}

export async function getErrorBook(token: string, dict?: string) {
  const qs = dict ? `?dict=${encodeURIComponent(dict)}` : ''
  return request<{ items: unknown[]; total: number }>(`/error-book${qs}`, { token })
}

export async function getSettings(token: string) {
  return request<UserSettings>('/settings', { token })
}

export async function patchSettings(token: string, patch: Partial<UserSettings>) {
  return request<UserSettings>('/settings', {
    method: 'PATCH',
    token,
    body: JSON.stringify(patch),
  })
}

// ---------------------------------------------------------------------------
// 迁移示意：打字页保存单词记录
// ---------------------------------------------------------------------------

export async function saveWordRecordAfterTyping(opts: {
  token: string
  word: string
  dictId: string
  chapter: number
  isReviewMode: boolean
  letterTimeArray: number[]
  wrongCount: number
  letterMistake: LetterMistakes
}) {
  const timing: number[] = []
  for (let i = 1; i < opts.letterTimeArray.length; i++) {
    timing.push(opts.letterTimeArray[i] - opts.letterTimeArray[i - 1])
  }

  return createWordRecord(opts.token, {
    word: opts.word,
    dict: opts.dictId,
    chapter: opts.isReviewMode ? -1 : opts.chapter,
    timing,
    wrongCount: opts.wrongCount,
    mistakes: opts.letterMistake,
  })
}
