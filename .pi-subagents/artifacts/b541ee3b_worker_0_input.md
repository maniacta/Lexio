# Task for worker

You are a delegated subagent running from a fork of the parent session. Treat the inherited conversation as reference-only context, not a live thread to continue. Do not continue or answer prior messages as if they are waiting for a reply. Your sole job is to execute the task below and return a focused result for that task using your tools.

Task:
你正在实现任务 1：数据库 Migration — 新增 4 张表

## 任务描述

先读你的任务简报：/home/ubuntu/Lexio/.worktrees/feat-settings/.superpowers/sdd/task-1-brief.md
它包含计划中该任务的完整文本。

## 上下文

这是 Lexio 设置功能实现的第 1 步。你需要在 SQLite 数据库 schema 中新增 4 张表（settings, model_providers, provider_models, task_models）。这些表将被后续任务使用（Task 3 的 repository 层和 Task 5 的 API 层）。

当前数据库 migration 在 `src-tauri/src/db.rs` 的 `migrate()` 方法中，使用 `CREATE TABLE IF NOT EXISTS` 模式。

工作目录：/home/ubuntu/Lexio/.worktrees/feat-settings

**现有模式：** 所有表定义在同一个 `execute_batch` 调用中，用分号分隔。

## 开始之前

如果你对需求有任何疑问，现在就问。

## 你的工作

1. 按照任务简报指定内容精确实现
2. 运行 `cd src-tauri && cargo check` 验证编译
3. 提交
4. 自审
5. 写出报告到 /home/ubuntu/Lexio/.worktrees/feat-settings/.superpowers/sdd/task-1-report.md

## 汇报格式

完整报告写到报告文件。最终消息仅包含：
- **状态：** DONE | DONE_WITH_CONCERNS | BLOCKED | NEEDS_CONTEXT
- 创建的提交（短 SHA + 标题）
- 一行测试小结
- 你的疑虑（如果有）

## Acceptance Contract
Acceptance level: reviewed
Completion is not accepted from prose alone. End with a structured acceptance report.

Criteria:
- criterion-1: Implement the requested change without widening scope
- criterion-2: Return evidence sufficient for an independent acceptance review

Required evidence: changed-files, tests-added, commands-run, validation-output, residual-risks, no-staged-files

Review gate: required by reviewer.

Finish with a fenced JSON block tagged `acceptance-report` in this shape:
Use empty arrays when no items apply; array fields contain strings unless object entries are shown.
`criteriaSatisfied[].status` must be exactly one of: satisfied, not-satisfied, not-applicable.
`commandsRun[].result` must be exactly one of: passed, failed, not-run.
`manualNotes` and `notes` are optional strings; an empty string means no note and does not satisfy `manual-notes` evidence.
```acceptance-report
{
  "criteriaSatisfied": [
    {
      "id": "criterion-1",
      "status": "satisfied",
      "evidence": "specific proof"
    },
    {
      "id": "criterion-2",
      "status": "satisfied",
      "evidence": "specific proof"
    }
  ],
  "changedFiles": [
    "src/file.ts"
  ],
  "testsAddedOrUpdated": [
    "test/file.test.ts"
  ],
  "commandsRun": [
    {
      "command": "command",
      "result": "passed",
      "summary": "short result"
    }
  ],
  "validationOutput": [
    "validation output or concise summary"
  ],
  "residualRisks": [
    "none"
  ],
  "noStagedFiles": true,
  "diffSummary": "short description of the diff",
  "reviewFindings": [
    "blocker: file.ts:12 - issue found, or no blockers"
  ],
  "manualNotes": "anything else the parent should know"
}
```